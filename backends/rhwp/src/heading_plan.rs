// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn

use md2hwp_core::{ValidatedDocument, ir::Block, metadata::heading1_start};

/// Manuscript state for one heading, indexed by its original block operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadingContext {
    pub level: u8,
    pub numbers: [u32; 6],
    /// First use of this heading role anywhere in the manuscript.
    pub first: bool,
    pub direct_children: Vec<usize>,
}

#[derive(Debug)]
pub struct HeadingPlan {
    contexts: Vec<Option<HeadingContext>>,
}

impl HeadingPlan {
    pub fn new(document: &ValidatedDocument) -> Result<Self, String> {
        let document = document.as_document();
        let start = heading1_start(&document.metadata)
            .map_err(|(path, message)| format!("{path}: {message}"))?;
        let mut contexts: Vec<Option<HeadingContext>> = vec![None; document.blocks.len()];
        let mut numbers = [0_u32; 6];
        let mut seen = [false; 6];
        // There can be at most six active ancestors. This keeps child discovery
        // linear even when the document contains many adjacent headings.
        let mut ancestors: Vec<(u8, usize)> = Vec::new();

        for (operation, block) in document.blocks.iter().enumerate() {
            let Block::Heading { level, .. } = block else {
                continue;
            };
            let index = usize::from(*level - 1);
            let first = !seen[index];
            numbers[index] = if *level == 1 && first {
                start
            } else {
                numbers[index]
                    .checked_add(1)
                    .filter(|number| *number <= i32::MAX as u32)
                    .ok_or_else(|| format!("Heading{level} numbering exceeds 2147483647"))?
            };
            numbers[index + 1..].fill(0);
            seen[index] = true;

            while ancestors.last().is_some_and(|(parent, _)| parent >= level) {
                ancestors.pop();
            }
            if let Some(&(parent_level, parent_operation)) = ancestors.last()
                && parent_level + 1 == *level
            {
                contexts[parent_operation]
                    .as_mut()
                    .expect("active ancestors have heading contexts")
                    .direct_children
                    .push(operation);
            }
            contexts[operation] = Some(HeadingContext {
                level: *level,
                numbers,
                first,
                direct_children: Vec::new(),
            });
            ancestors.push((*level, operation));
        }
        Ok(Self { contexts })
    }

    pub fn context(&self, operation: usize) -> Option<&HeadingContext> {
        self.contexts.get(operation).and_then(Option::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use md2hwp_core::{
        Document, ValidationLimits,
        ir::{IR_VERSION, Inline, Metadata, MetadataValue, SCHEMA_NAME},
        validate,
    };

    fn heading(level: u8, id: Option<&str>) -> Block {
        Block::Heading {
            level,
            id: id.map(str::to_owned),
            inlines: vec![Inline::Text {
                value: "제목".into(),
            }],
        }
    }

    fn document(blocks: Vec<Block>, start: Option<&str>) -> ValidatedDocument {
        let mut metadata = Metadata::default();
        if let Some(start) = start {
            metadata.0.insert(
                "md2hwp-heading1-start".into(),
                MetadataValue::Text(start.into()),
            );
        }
        validate(
            Document {
                schema: SCHEMA_NAME.into(),
                ir_version: IR_VERSION.into(),
                metadata,
                blocks,
            },
            &ValidationLimits::default(),
        )
        .unwrap()
    }

    #[test]
    fn counters_include_idless_headings_and_reset_only_deeper_levels() {
        let plan = HeadingPlan::new(&document(
            vec![
                heading(3, None),
                heading(1, None),
                heading(2, None),
                heading(3, Some("세부")),
                heading(2, None),
                heading(1, Some("다음")),
                heading(4, None),
            ],
            Some("7"),
        ))
        .unwrap();
        let expected = [
            [0, 0, 1, 0, 0, 0],
            [7, 0, 0, 0, 0, 0],
            [7, 1, 0, 0, 0, 0],
            [7, 1, 1, 0, 0, 0],
            [7, 2, 0, 0, 0, 0],
            [8, 0, 0, 0, 0, 0],
            [8, 0, 0, 1, 0, 0],
        ];
        for (operation, numbers) in expected.into_iter().enumerate() {
            assert_eq!(plan.context(operation).unwrap().numbers, numbers);
        }
        assert!(plan.context(0).unwrap().first);
        assert!(!plan.context(3).unwrap().first);
        assert!(!plan.context(5).unwrap().first);
    }

    #[test]
    fn children_use_original_operation_indices_and_exact_next_level() {
        let plan = HeadingPlan::new(&document(
            vec![
                heading(1, None),
                Block::Paragraph {
                    inlines: vec![Inline::Text {
                        value: "본문".into(),
                    }],
                },
                heading(2, None),
                heading(3, None),
                heading(4, None),
                heading(2, None),
                heading(4, None),
                heading(3, None),
                heading(1, None),
                heading(3, None),
            ],
            None,
        ))
        .unwrap();
        assert_eq!(plan.context(0).unwrap().direct_children, vec![2, 5]);
        assert_eq!(plan.context(2).unwrap().direct_children, vec![3]);
        assert_eq!(plan.context(3).unwrap().direct_children, vec![4]);
        assert_eq!(plan.context(5).unwrap().direct_children, vec![7]);
        assert!(plan.context(8).unwrap().direct_children.is_empty());
        assert_eq!(plan.context(0).unwrap().numbers, [1, 0, 0, 0, 0, 0]);
        assert!(plan.context(1).is_none());
        assert!(plan.context(10).is_none());
        assert!(plan.context(usize::MAX).is_none());
    }

    #[test]
    fn maximum_heading1_start_is_not_incremented_before_first_heading() {
        let plan = HeadingPlan::new(&document(
            vec![heading(2, None), heading(1, None)],
            Some("2147483647"),
        ))
        .unwrap();
        assert_eq!(
            plan.context(1).unwrap().numbers,
            [2147483647, 0, 0, 0, 0, 0]
        );
    }
}
