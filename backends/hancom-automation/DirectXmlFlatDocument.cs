using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Construct the unpublished intermediate from the validated IR and template.
// The live document supplies only picture resources produced by InsertPicture.
internal sealed record DirectXmlFlatDocument(XDocument Document, int Start, DirectXmlLists Lists)
{
    internal static DirectXmlFlatDocument Compose(XDocument template, XDocument resources,
        TaggedTemplateBinding binding, AuriPreviewStyleBindings styles, IrPreviewPlan plan,
        IReadOnlyList<XElement> pictures, bool verbose = false, int? zOrderStart = null)
    {
        if (pictures.Count != plan.Summary.FigureOperations)
            throw new InvalidOperationException("Picture resource count differs from the IR plan.");
        var result = new XDocument(resources);
        var originals = AuriMinimalBoxPrototype.RootParagraphs(template);
        TemplateRangeStructure.RequireOriginalStyleDefinitions(template, resources, originals);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(result);
        var section = roots[0].Parent!;
        // Never take static content from the resource-generation result.
        // Its expected structure comes directly from the template snapshot.
        section.ReplaceNodes(originals.Take(binding.TemplateBegin).Select(p =>
        {
            var imported = TemplateHeadingBlocks.ImportParagraph(p, template, result);
            // Static fields may refer to these identities. Remap formats only;
            // unlike generated clones, retain all original static identities.
            var identities = p.DescendantsAndSelf().Attributes().Where(a => a.Name.LocalName is "InstId" or "InstID").ToArray();
            var replacements = imported.DescendantsAndSelf().Attributes().Where(a => a.Name.LocalName is "InstId" or "InstID").ToArray();
            for (var i = 0; i < identities.Length; i++) replacements[i].Value = identities[i].Value;
            return imported;
        }).ToArray());
        var paragraphs = new DirectXmlParagraphs(result, styles);
        var originalLists = styles.Profile.Lists;
        var listFormats = originalLists with
        {
            Bullet = new TemplateListPrototype("bullet", 0,
                TemplateHeadingBlocks.ImportParagraph(originalLists.Prototype("bullet").Definition, template, result)),
            Ordered = new TemplateListPrototype("ordered", 0,
                TemplateHeadingBlocks.ImportParagraph(originalLists.Prototype("ordered").Definition, template, result)),
        };
        var lists = new DirectXmlLists(result, plan, listFormats);
        var pictureIndex = 0;
        var nextZOrder = zOrderStart;
        if (zOrderStart is { } origin)
        {
            // Native HWP insertion/materialization assigns body objects before
            // master-page objects. Retain the template's content and stacking,
            // but reserve room for every generated code/picture in that native
            // allocation order rather than reuse stale pre-insertion numbers.
            var staticShapes = section.Descendants("SHAPEOBJECT").ToArray();
            var resourceShapes = roots.Take(binding.TemplateBegin).SelectMany(root => root.Descendants("SHAPEOBJECT")).ToArray();
            if (staticShapes.Length != resourceShapes.Length ||
                staticShapes.Zip(resourceShapes).Any(pair => pair.First.Parent!.Name != pair.Second.Parent!.Name))
                throw new InvalidOperationException("Picture resource collection changed static object allocation structure.");
            var collected = pictures.Count == 0 ? 1 : pictures.Count;
            var added = checked(plan.Summary.BoxOperations + pictures.Count - collected);
            foreach (var (shape, resource) in staticShapes.Zip(resourceShapes))
            {
                var order = (int?)resource.Attribute("ZOrder") ??
                    throw new InvalidOperationException("Static native object has no insertion order.");
                shape.SetAttributeValue("ZOrder", order >= origin ? checked(order + added) : order);
            }
        }
        foreach (var operation in plan.Operations)
        {
            if (verbose) Console.Error.WriteLine($"ir2hwp: {operation.Kind}/{operation.Label}");
#if DEBUG
            using var timing = RenderProfile.Measure("compose." + operation.Kind);
#endif
            if (operation.Kind is "text" or "table")
            {
                var generated = paragraphs.Create(operation);
                foreach (var paragraph in generated)
                {
                    if (operation.ListMarker is { } marker)
                        lists.Apply(paragraph, marker, styles.Resolve(operation.ParagraphStyle!).BaseLeftMargin);
                    else
                    {
                        lists.BreakSequence();
                        if (operation.ListContinuation is { } continuation)
                            lists.ApplyContinuation(paragraph, continuation);
                    }
                    section.Add(paragraph);
                }
            }
            else if (operation.Kind == "code")
            {
                lists.BreakSequence();
                var root = TemplateHeadingBlocks.ImportParagraph(originals[binding.BoxRoot], template, result);
                var table = root.Descendants("TABLE").Single();
                Order(table);
                var content = table.Descendants("CELL").Single().Descendants("P").Single();
                TemplateTables.FillSlot(content, TaggedTemplateBinding.Tag("slot:code.content"),
                    PreviewInlineContent.Plain(operation.Lines), result);
                var caption = table.Element("SHAPEOBJECT")!.Element("CAPTION")!;
                if (operation.SourceRuns is { } source)
                    TemplateTables.FillSlot(caption.Descendants("P").Single(), styles.Profile.BoxSource.Slot,
                        Content(source), result);
                else caption.Remove();
                section.Add(root);
            }
            else if (operation.Kind == "figure")
            {
                lists.BreakSequence();
                var root = paragraphs.Create("figure", []);
                var picture = new XElement(pictures[pictureIndex++]);
                Order(picture);
                root.Element("TEXT")!.ReplaceNodes(picture, new XElement("CHAR", ""));
                section.Add(root);
                var caption = TemplateHeadingBlocks.ImportParagraph(originals[binding.CaptionRoot], template, result);
                var editingFormat = SlotCharacterShape(caption, styles.Profile.CaptionSelector.PrototypeCaption);
                TemplateTables.FillSlot(caption, styles.Profile.CaptionSelector.PrototypeCaption,
                    Content(operation.FormattedLines![1]), result);
                // The native text renderer restores the slot's editing format
                // after inserting a marked final run, leaving an empty baseline
                // run. Retain that state as well as the visible caption.
                if ((string?)caption.Elements("TEXT").Last().Attribute("CharShape") != editingFormat)
                    caption.Add(new XElement("TEXT", new XAttribute("CharShape", editingFormat), new XElement("CHAR", "")));
                section.Add(caption);
                if (operation.Lines[2].Length > 0)
                {
                    var source = TemplateHeadingBlocks.ImportParagraph(originals[binding.CaptionRoot + 1], template, result);
                    TemplateTables.FillSlot(source, styles.Profile.FigureSource.Slot,
                        Content(operation.FormattedLines[2]), result);
                    section.Add(source);
                }
            }
            else throw new InvalidOperationException($"Unsupported direct XML operation: {operation.Kind}");
        }
        lists.BreakSequence();
        var terminal = paragraphs.Create("body", []);
        DirectXmlLists.Clear(terminal, result);
        section.Add(terminal);
        return new(result, binding.TemplateBegin, lists);

        void Order(XElement nativeObject)
        {
            if (nextZOrder is not { } value) return;
            nativeObject.Element("SHAPEOBJECT")!.SetAttributeValue("ZOrder", value);
            nextZOrder = checked(value + 1);
        }
    }

    private static string SlotCharacterShape(XElement paragraph, string slot)
    {
        var offset = TaggedTemplateBinding.DirectText(paragraph).IndexOf(slot, StringComparison.Ordinal);
        if (offset < 0) throw new InvalidOperationException("Missing direct caption slot.");
        foreach (var character in paragraph.Elements("TEXT").Elements("CHAR"))
        {
            if (offset < character.Value.Length)
                return (string?)character.Parent!.Attribute("CharShape")
                    ?? throw new InvalidOperationException("Caption slot has no editing format.");
            offset -= character.Value.Length;
        }
        throw new InvalidOperationException("Cannot locate direct caption editing format.");
    }

    private static PreviewInlineContent Content(IReadOnlyList<PreviewTextRun> runs) =>
        new([new PreviewLine(string.Concat(runs.Select(run => run.Text)), runs)]);
}
