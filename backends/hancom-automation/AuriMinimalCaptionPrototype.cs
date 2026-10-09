using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record CaptionInsertion(int RootParagraphIndex);

internal sealed class AuriMinimalCaptionPrototype
{
    private readonly int originalRootParagraphIndex;
    private readonly string originalRootXml;
    private readonly string originalStructureXml;
    private readonly string selectedBlockHwp;

    private AuriMinimalCaptionPrototype(
        int originalRootParagraphIndex,
        string originalRootXml,
        string originalStructureXml,
        string selectedBlockHwp)
    {
        this.originalRootParagraphIndex = originalRootParagraphIndex;
        this.originalRootXml = originalRootXml;
        this.originalStructureXml = originalStructureXml;
        this.selectedBlockHwp = selectedBlockHwp;
    }

    public static AuriMinimalCaptionPrototype Bind(
        dynamic hwp,
        AuriPreviewStyleBindings styles)
    {
        XDocument document = ReadDocument(hwp);
        IReadOnlyList<XElement> roots = RootParagraphs(document);
        var candidates = FindCandidates(roots, styles, requirePrototypeCaption: true);
        if (candidates.Count != 1)
        {
            throw new InvalidOperationException(
                $"Expected one minimal-fixture figure-caption prototype, found {candidates.Count}.");
        }

        var candidate = candidates[0];
        if (!(bool)hwp.SetPos(0, candidate.RootParagraphIndex, 0))
        {
            throw new InvalidOperationException(
                $"Hancom could not select caption prototype paragraph {candidate.RootParagraphIndex}.");
        }
        Run(hwp, "MoveParaBegin");
        Run(hwp, "MoveSelNextParaBegin");
        var selectedBlock = (string)hwp.GetTextFile("HWPML2X", "saveblock");
        var selectedNativeBlock = (string)hwp.GetTextFile("HWP", "saveblock");
        Run(hwp, "Cancel");
        if (string.IsNullOrWhiteSpace(selectedBlock))
        {
            throw new InvalidOperationException("Hancom returned an empty caption prototype block.");
        }
        if (string.IsNullOrWhiteSpace(selectedNativeBlock))
        {
            throw new InvalidOperationException("Hancom returned an empty native HWP caption block.");
        }

        var selectedDocument = HwpMarkup.Parse(selectedBlock);
        ValidateSelectedBlock(selectedDocument, styles.Profile.CaptionSelector);

        return new AuriMinimalCaptionPrototype(
            candidate.RootParagraphIndex,
            StableCaptionContentXml(candidate.Root),
            StableCaptionStructureXml(candidate.Root, styles),
            selectedNativeBlock);
    }

    public CaptionInsertion Insert(
        dynamic hwp,
        AuriPreviewStyleBindings styles,
        IReadOnlyList<PreviewTextRun> captionRuns)
    {
        var caption = string.Concat(captionRuns.Select(run => run.Text));
        if (caption.Length == 0 || caption.Contains('\r') || caption.Contains('\n'))
        {
            throw new InvalidOperationException(
                "A figure caption requires nonempty CR/LF-free formatted text.");
        }

#if DEBUG
        XDocument beforeDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> beforeRoots = RootParagraphs(beforeDocument);
        VerifyOriginal(beforeRoots);
        var beforeRootXml = beforeRoots
            .Select(root => StableRootSequenceXml(root, styles))
            .ToArray();
        var tablesBefore = Count(beforeDocument, "TABLE");
        var picturesBefore = Count(beforeDocument, "PICTURE");
        var figureNumbersBefore = CountFigureAutoNumbers(beforeDocument);
#endif

        Run(hwp, "Cancel");
        Run(hwp, "MoveDocEnd");
#if !DEBUG
        var clonePosition = NativeClonePosition.Begin((object)hwp);
#endif
        object? insertionResult = hwp.SetTextFile(
            selectedBlockHwp,
            "HWP",
            "insertfile");
        if (insertionResult is bool booleanResult && !booleanResult)
        {
            throw new InvalidOperationException("Hancom rejected the native HWP caption insertion.");
        }

#if DEBUG
        XDocument afterDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> afterRoots = RootParagraphs(afterDocument);
        var afterRootXml = afterRoots
            .Select(root => StableRootSequenceXml(root, styles))
            .ToArray();
        var captionCandidates = FindCandidates(
            afterRoots,
            styles,
            requirePrototypeCaption: true);
        var insertionCandidates = captionCandidates
            .Where(candidate => candidate.RootParagraphIndex != originalRootParagraphIndex)
            .Where(candidate => beforeRootXml.SequenceEqual(
                afterRootXml.Where((_, index) => index != candidate.RootParagraphIndex),
                StringComparer.Ordinal))
            .OrderBy(candidate => candidate.RootParagraphIndex)
            .ToArray();
        if (afterRoots.Count != beforeRoots.Count + 1 || insertionCandidates.Length == 0)
        {
            throw new InvalidOperationException(
                "Caption insertion did not add one removable automatic-number prototype " +
                $"at the document end; roots={beforeRoots.Count}->{afterRoots.Count}, " +
                $"candidates=[{string.Join(",", captionCandidates.Select(candidate =>
                    candidate.RootParagraphIndex))}], " +
                $"reconstruction=[{string.Join(";", captionCandidates
                    .Where(candidate => candidate.RootParagraphIndex != originalRootParagraphIndex)
                    .Select(candidate =>
                        $"{candidate.RootParagraphIndex}:" + DescribeSequenceDifference(
                            beforeRootXml,
                            afterRootXml.Where((_, index) =>
                                index != candidate.RootParagraphIndex).ToArray())))}], " +
                $"result={insertionResult ?? "null"} " +
                $"({insertionResult?.GetType().FullName ?? "null"}).");
        }

        // Adjacent prototype-identical captions can both reconstruct the old
        // sequence. MoveDocEnd makes the last matching root the new clone.
        var insertedIndex = insertionCandidates[^1].RootParagraphIndex;
        var insertedRootXml = StableCaptionContentXml(afterRoots[insertedIndex]);
        var currentOriginalXml = StableCaptionContentXml(
            afterRoots[originalRootParagraphIndex]);
        if (!string.Equals(insertedRootXml, currentOriginalXml, StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "The inserted native caption differed from its prototype before text replacement: " +
                DescribeFirstDifference(currentOriginalXml, insertedRootXml));
        }
        if (insertedIndex <= originalRootParagraphIndex)
        {
            throw new InvalidOperationException(
                "The caption was not inserted after its source prototype.");
        }
        VerifyOriginal(afterRoots);
        if (Count(afterDocument, "TABLE") != tablesBefore ||
            Count(afterDocument, "PICTURE") != picturesBefore ||
            CountFigureAutoNumbers(afterDocument) != figureNumbersBefore + 1)
        {
            throw new InvalidOperationException(
                "Caption insertion did not add exactly one figure automatic number " +
                "while preserving tables and pictures.");
        }
#else
        var insertedIndex = NativeClonePosition.Complete((object)hwp, clonePosition);
        if (insertedIndex <= originalRootParagraphIndex)
            throw new InvalidOperationException("The caption was not inserted after its source prototype.");
#endif

        ReplaceCloneCaption(hwp, styles, insertedIndex, captionRuns);

        Run(hwp, "MoveDocEnd");
        return new CaptionInsertion(insertedIndex);
    }

    public void VerifyOriginal(IReadOnlyList<XElement> roots)
    {
        if (originalRootParagraphIndex >= roots.Count ||
            !string.Equals(
                StableCaptionContentXml(roots[originalRootParagraphIndex]),
                originalRootXml,
                StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "The source caption prototype changed during preview rendering.");
        }
    }

    public void VerifyRenderedRoot(
        XElement root,
        AuriPreviewStyleBindings styles,
        string caption)
    {
        if (!TryReadParts(
                root,
                styles,
                requirePrototypeCaption: false,
                out var parts) ||
            !string.Equals(parts.Caption, caption, StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                $"Rendered automatic-number caption did not contain the expected text: {caption}");
        }
        var structure = StableCaptionStructureXml(root, styles);
        if (!string.Equals(structure, originalStructureXml, StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "Rendered caption structure differed from the prototype outside its text runs: " +
                DescribeFirstDifference(originalStructureXml, structure));
        }
    }

    public static XDocument ReadDocument(dynamic hwp) =>
        HwpMarkup.ReadDocument((object)hwp);

    public static IReadOnlyList<XElement> RootParagraphs(XDocument document) =>
        AuriMinimalBoxPrototype.RootParagraphs(document);

    public static int CountFigureAutoNumbers(XDocument document) =>
        document.Descendants().Count(element =>
            element.Name.LocalName == "AUTONUM" &&
            string.Equals(
                element.Attribute("NumberType")?.Value,
                "Figure",
                StringComparison.Ordinal));

    private void ReplaceCloneCaption(
        dynamic hwp,
        AuriPreviewStyleBindings styles,
        int cloneRootParagraphIndex,
        IReadOnlyList<PreviewTextRun> captionRuns)
    {
        var sentinel = $"MD2HWP_CAPTION_{Guid.NewGuid():N}";
        // Clone discovery still reads the document in both builds. Replacement
        // diagnostics repeat the completed-document checks and belong to Debug.
#if DEBUG
        XDocument beforeDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> beforeRoots = RootParagraphs(beforeDocument);
        VerifyOriginal(beforeRoots);
        if ((beforeDocument.Root?.Value ?? string.Empty).Contains(
                sentinel,
                StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "Generated caption sentinel already existed in the document.");
        }

#endif
        MoveToRoot(hwp, cloneRootParagraphIndex);
        FindNext(hwp, styles.Profile.CaptionSelector.PrototypeCaption);
        InsertText(hwp, sentinel);

#if DEBUG
        XDocument sentinelDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> sentinelRoots = RootParagraphs(sentinelDocument);
        VerifyOriginal(sentinelRoots);
        if (CountOccurrences(sentinelDocument.Root?.Value ?? string.Empty, sentinel) != 1)
        {
            throw new InvalidOperationException(
                "Caption sentinel was not isolated to exactly one cloned paragraph.");
        }
        VerifyRenderedRoot(
            sentinelRoots[cloneRootParagraphIndex],
            styles,
            sentinel);

#endif
        MoveToRoot(hwp, cloneRootParagraphIndex);
        FindNext(hwp, sentinel);
        // Character-shape toggles apply to an active find selection instead of
        // the replacement text in HWP 2020. Remove the isolated sentinel first
        // so formatted insertion starts at an ordinary base-style caret.
        Run(hwp, "Delete");
        HancomPreviewWriter.InsertFormattedLine(
            hwp,
            captionRuns,
            styles.Resolve(styles.Profile.CaptionSelector.ParagraphStyle));

        var caption = string.Concat(captionRuns.Select(run => run.Text));
        HancomPreviewWriter.RemoveHyperlinksInRoots(hwp, cloneRootParagraphIndex, cloneRootParagraphIndex + 1);
#if DEBUG
        XDocument finalDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> finalRoots = RootParagraphs(finalDocument);
        VerifyOriginal(finalRoots);
        if ((finalDocument.Root?.Value ?? string.Empty).Contains(
                sentinel,
                StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "Caption sentinel remained after content replacement.");
        }
        VerifyRenderedRoot(finalRoots[cloneRootParagraphIndex], styles, caption);
#endif

    }

    private static IReadOnlyList<CaptionCandidate> FindCandidates(
        IReadOnlyList<XElement> roots,
        AuriPreviewStyleBindings styles,
        bool requirePrototypeCaption)
    {
        var result = new List<CaptionCandidate>();
        for (var index = 0; index < roots.Count; index++)
        {
            if (TryReadParts(roots[index], styles, requirePrototypeCaption, out var parts))
            {
                result.Add(new CaptionCandidate(index, roots[index], parts));
            }
        }
        return result;
    }

    private static bool TryReadParts(
        XElement root,
        AuriPreviewStyleBindings styles,
        bool requirePrototypeCaption,
        out CaptionParts parts)
    {
        parts = null!;
        var selector = styles.Profile.CaptionSelector;
        if (ReadStyle(root) != styles.Resolve(selector.ParagraphStyle).Id ||
            root.Descendants().Any(element =>
                element.Name.LocalName is "TABLE" or "PICTURE" or "LINEBREAK"))
        {
            return false;
        }

        var textElements = root.Elements()
            .Where(element => element.Name.LocalName == "TEXT")
            .ToArray();
        if (textElements.Length == 0 || root.Elements().Count() != textElements.Length)
        {
            return false;
        }
        var contentElements = textElements.SelectMany(text => text.Elements()).ToArray();
        if (contentElements.Length == 0 ||
            contentElements.Any(element => element.Name.LocalName is not ("CHAR" or "AUTONUM")))
        {
            return false;
        }
        var autoNumbers = contentElements
            .Where(element => element.Name.LocalName == "AUTONUM")
            .ToArray();
        if (autoNumbers.Length != 1 || !IsFigureAutoNumber(autoNumbers[0]))
        {
            return false;
        }
        if (contentElements
            .Where(element => element.Name.LocalName == "CHAR")
            .Any(character => character.Nodes().Any(node => node is not XText)))
        {
            return false;
        }

        var beforeNumber = new System.Text.StringBuilder();
        var afterNumber = new System.Text.StringBuilder();
        var seenNumber = false;
        foreach (var element in contentElements)
        {
            if (element.Name.LocalName == "AUTONUM")
            {
                seenNumber = true;
                continue;
            }
            (seenNumber ? afterNumber : beforeNumber).Append(element.Value);
        }
        if (!selector.TryReadCaption(beforeNumber.ToString(), afterNumber.ToString(), out var caption))
            return false;
        if (caption.Length == 0 ||
            (requirePrototypeCaption &&
             !string.Equals(
                 caption,
                 selector.PrototypeCaption,
                 StringComparison.Ordinal)))
        {
            return false;
        }
        parts = new CaptionParts(autoNumbers[0], caption);
        return true;
    }

    internal static bool IsFigureAutoNumber(XElement autoNumber)
    {
        var attributes = autoNumber.Attributes().ToArray();
        if (attributes.Length != 2 ||
            !attributes.Any(attribute =>
                attribute.Name.LocalName == "NumberType" &&
                string.Equals(attribute.Value, "Figure", StringComparison.Ordinal)) ||
            !attributes.Any(attribute =>
                attribute.Name.LocalName == "Number" &&
                int.TryParse(attribute.Value, out var number) && number >= 0))
        {
            return false;
        }
        var formats = autoNumber.Elements().ToArray();
        if (formats.Length != 1 || formats[0].Name.LocalName != "AUTONUMFORMAT")
        {
            return false;
        }
        return !formats[0].HasElements && !string.IsNullOrEmpty((string?)formats[0].Attribute("Type"));
    }

    private static void ValidateSelectedBlock(
        XDocument selectedDocument,
        ProfileCaptionSelector selector)
    {
        var sections = selectedDocument.Descendants()
            .Where(element => element.Name.LocalName == "SECTION")
            .ToArray();
        var autoNumbers = selectedDocument.Descendants()
            .Where(element => element.Name.LocalName == "AUTONUM")
            .ToArray();
        if (sections.Length != 1 ||
            autoNumbers.Length != 1 ||
            !IsFigureAutoNumber(autoNumbers[0]))
        {
            throw new InvalidOperationException(
                "The selected caption block did not preserve exactly one Figure AUTONUM control.");
        }

        // HWP 2020 saveblock embeds the selected paragraph content in the
        // SECTION-definition TEXT instead of returning its ordinary root-P
        // representation. Read text across character-formatting runs;
        // the inserted native block is compared with the ordinary prototype
        // again after SetTextFile(..., \"HWP\", \"insertfile\").
        var elements = sections[0].Descendants().Where(e => e.Name.LocalName is "CHAR" or "AUTONUM").ToArray();
        var number = autoNumbers[0];
        var before = string.Concat(elements.TakeWhile(e => e != number).Where(e => e.Name.LocalName == "CHAR").Select(e => e.Value));
        var after = string.Concat(elements.SkipWhile(e => e != number).Skip(1).Where(e => e.Name.LocalName == "CHAR").Select(e => e.Value));
        if (!selector.TryReadCaption(before, after, out var caption) || caption != selector.PrototypeCaption)
            throw new InvalidOperationException("The selected caption block changed its template text or slot.");

    }

    private static string StableRootSequenceXml(
        XElement root,
        AuriPreviewStyleBindings styles) =>
        TryReadParts(root, styles, requirePrototypeCaption: false, out _)
            ? StableCaptionContentXml(root)
            : StableAdjacentRootXml(root, styles);

    private static string StableAdjacentRootXml(
        XElement root,
        AuriPreviewStyleBindings styles)
    {
        var clone = TemplateFormatting.Copy(root);
        // InsertFile makes HWP 2020 materialize a just-inserted picture's
        // previously-zero rotation centre. The picture geometry and transform
        // remain unchanged, so exclude only this lazy derived pair while
        // proving that the caption clone is the sole new semantic root.
        foreach (var rotation in clone.DescendantsAndSelf()
                     .Where(element => element.Name.LocalName == "ROTATIONINFO"))
        {
            rotation.Attribute("CenterX")?.Remove();
            rotation.Attribute("CenterY")?.Remove();
        }
        return AuriMinimalBoxPrototype.StableRootSequenceXml(clone, styles);
    }

    private static string StableCaptionContentXml(XElement root)
    {
        var clone = TemplateFormatting.Copy(root);
        foreach (var autoNumber in clone.Descendants()
                     .Where(element => element.Name.LocalName == "AUTONUM"))
        {
            autoNumber.Attributes()
                .FirstOrDefault(attribute => attribute.Name.LocalName == "Number")
                ?.Remove();
        }
        return StableXml(clone);
    }

    private static string StableCaptionStructureXml(
        XElement root,
        AuriPreviewStyleBindings styles)
    {
        var clone = TemplateFormatting.Copy(root);
        if (!TryReadParts(clone, styles, requirePrototypeCaption: false, out var parts))
        {
            throw new InvalidOperationException(
                "Could not normalize the minimal-fixture caption structure for comparison.");
        }
        var autoNumber = new XElement(parts.AutoNumber);
        autoNumber.Attributes()
            .First(attribute => attribute.Name.LocalName == "Number")
            .Remove();
        clone.RemoveNodes();
        clone.Add(autoNumber);
        return StableXml(clone);
    }

    private static string StableXml(XElement element)
    {
        var clone = TemplateFormatting.Copy(element);
        foreach (var attribute in clone.DescendantsAndSelf()
                     .Attributes()
                     .Where(attribute => attribute.Name.LocalName is "InstId" or "ZOrder")
                     .ToArray())
        {
            attribute.Remove();
        }
        return clone.ToString(SaveOptions.DisableFormatting);
    }

    private static void MoveToRoot(dynamic hwp, int rootParagraphIndex)
    {
        if (!(bool)hwp.SetPos(0, rootParagraphIndex, 0))
        {
            throw new InvalidOperationException(
                $"Hancom could not move to caption root paragraph {rootParagraphIndex}.");
        }
        Run(hwp, "MoveParaBegin");
    }

    private static void FindNext(dynamic hwp, string text)
    {
        _ = hwp.HAction.GetDefault("RepeatFind", hwp.HParameterSet.HFindReplace.HSet);
        hwp.HParameterSet.HFindReplace.FindString = text;
        hwp.HParameterSet.HFindReplace.Direction = hwp.FindDir("Forward");
        hwp.HParameterSet.HFindReplace.IgnoreMessage = 1;
        if (!(bool)hwp.HAction.Execute(
                "RepeatFind",
                hwp.HParameterSet.HFindReplace.HSet))
        {
            throw new InvalidOperationException(
                $"Hancom could not find caption text: {text}");
        }
    }

    private static void InsertText(dynamic hwp, string text) =>
        HancomPreviewWriter.InsertText(hwp, text);

    private static int CountOccurrences(string source, string value)
    {
        var count = 0;
        var start = 0;
        while ((start = source.IndexOf(value, start, StringComparison.Ordinal)) >= 0)
        {
            count++;
            start += value.Length;
        }
        return count;
    }

    private static int ReadStyle(XElement paragraph) =>
        int.TryParse(paragraph.Attribute("Style")?.Value, out var style)
            ? style
            : -1;

    private static int Count(XContainer container, string localName) =>
        container.Descendants().Count(element => element.Name.LocalName == localName);

    private static string DescribeFirstDifference(string before, string after)
    {
        var index = 0;
        while (index < Math.Min(before.Length, after.Length) && before[index] == after[index])
        {
            index++;
        }
        var beforeStart = Math.Max(0, index - 40);
        var afterStart = Math.Max(0, index - 40);
        return $"at={index},before={System.Text.Json.JsonSerializer.Serialize(
            before.Substring(beforeStart, Math.Min(160, before.Length - beforeStart)))}," +
            $"after={System.Text.Json.JsonSerializer.Serialize(
                after.Substring(afterStart, Math.Min(160, after.Length - afterStart)))}";
    }

    private static string DescribeSequenceDifference(
        IReadOnlyList<string> before,
        IReadOnlyList<string> after)
    {
        if (before.Count != after.Count)
        {
            return $"count={before.Count}->{after.Count}";
        }
        for (var index = 0; index < before.Count; index++)
        {
            if (!string.Equals(before[index], after[index], StringComparison.Ordinal))
            {
                return $"root={index}," + DescribeFirstDifference(before[index], after[index]);
            }
        }
        return "equal";
    }

    private static void Run(dynamic hwp, string action)
    {
        if (!(bool)hwp.HAction.Run(action))
        {
            throw new InvalidOperationException($"Hancom action failed: {action}");
        }
    }

    private sealed record CaptionCandidate(
        int RootParagraphIndex,
        XElement Root,
        CaptionParts Parts);

    private sealed record CaptionParts(
        XElement AutoNumber,
        string Caption);
}
