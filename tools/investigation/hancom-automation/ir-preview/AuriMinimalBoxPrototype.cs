using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record BoxInsertion(int RootParagraphIndex);

internal sealed class AuriMinimalBoxPrototype
{
    private const string PrototypeTextMarker = "스타일 블록 예시:";
    private readonly int originalRootParagraphIndex;
    private readonly string originalRootXml;
    private readonly string originalRootStructureXml;
    private readonly string selectedBlockHwp;

    private AuriMinimalBoxPrototype(
        int originalRootParagraphIndex,
        string originalRootXml,
        string originalRootStructureXml,
        string selectedBlockHwp)
    {
        this.originalRootParagraphIndex = originalRootParagraphIndex;
        this.originalRootXml = originalRootXml;
        this.originalRootStructureXml = originalRootStructureXml;
        this.selectedBlockHwp = selectedBlockHwp;
    }

    public static AuriMinimalBoxPrototype Bind(
        dynamic hwp,
        AuriPreviewStyleBindings styles)
    {
        XDocument document = ReadDocument(hwp);
        IReadOnlyList<XElement> roots = RootParagraphs(document);
        var candidates = FindCandidates(roots, styles, requirePrototypeLineBreak: true);
        if (candidates.Count != 1)
        {
            throw new InvalidOperationException(
                $"Expected one minimal-fixture box prototype, found {candidates.Count}.");
        }

        var candidate = candidates[0];
        if (!(bool)hwp.SetPos(0, candidate.RootParagraphIndex, 0))
        {
            throw new InvalidOperationException(
                $"Hancom could not select box prototype paragraph {candidate.RootParagraphIndex}.");
        }
        Run(hwp, "MoveParaBegin");
        Run(hwp, "MoveSelNextParaBegin");
        var selectedBlock = (string)hwp.GetTextFile("HWPML2X", "saveblock");
        var selectedNativeBlock = (string)hwp.GetTextFile("HWP", "saveblock");
        Run(hwp, "Cancel");
        if (string.IsNullOrWhiteSpace(selectedBlock))
        {
            throw new InvalidOperationException("Hancom returned an empty box prototype block.");
        }
        if (string.IsNullOrWhiteSpace(selectedNativeBlock))
        {
            throw new InvalidOperationException("Hancom returned an empty native HWP box block.");
        }

        var selectedDocument = XDocument.Parse(selectedBlock);
        var selectedStyles = AuriPreviewStyleBindings.BindDocument(selectedDocument);
        var selectedRoots = RootParagraphs(selectedDocument);
        var selectedCandidates = FindCandidates(
            selectedRoots,
            selectedStyles,
            requirePrototypeLineBreak: true);
        if (selectedCandidates.Count != 1)
        {
            throw new InvalidOperationException(
                $"The selected HWPML block contained {selectedRoots.Count} root paragraphs " +
                $"and {selectedCandidates.Count} minimal-fixture box candidates; expected one candidate. " +
                string.Join("; ", selectedRoots.Select((root, index) =>
                    $"root[{index}]={DescribeRoot(root)}")));
        }
        return new AuriMinimalBoxPrototype(
            candidate.RootParagraphIndex,
            StableXml(candidate.Root),
            StableBoxStructureXml(candidate.Root, styles),
            selectedNativeBlock);
    }

    public BoxInsertion Insert(
        dynamic hwp,
        AuriPreviewStyleBindings styles,
        IReadOnlyList<string> lines)
    {
        if (lines.Count == 0 || lines.Any(line => line.Contains('\r') || line.Contains('\n')))
        {
            throw new InvalidOperationException(
                "A box requires at least one CR/LF-free logical line.");
        }

        XDocument beforeDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> beforeRoots = RootParagraphs(beforeDocument);
        VerifyOriginal(beforeRoots);
        var beforeRootXml = beforeRoots
            .Select(root => StableRootSequenceXml(root, styles))
            .ToArray();
        var tablesBefore = Count(beforeDocument, "TABLE");
        var picturesBefore = Count(beforeDocument, "PICTURE");
        var autoNumbersBefore = Count(beforeDocument, "AUTONUM");

        Run(hwp, "Cancel");
        Run(hwp, "MoveDocEnd");
        object? insertionResult = hwp.SetTextFile(
            selectedBlockHwp,
            "HWP",
            "insertfile");
        if (insertionResult is bool booleanResult && !booleanResult)
        {
            throw new InvalidOperationException("Hancom rejected the native HWP box insertion.");
        }

        XDocument afterDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> afterRoots = RootParagraphs(afterDocument);
        var afterRootXml = afterRoots
            .Select(root => StableRootSequenceXml(root, styles))
            .ToArray();
        var boxesAfterInsertion = FindCandidates(
            afterRoots,
            styles,
            requirePrototypeLineBreak: true);
        var insertionCandidates = boxesAfterInsertion
            .Where(candidate => candidate.RootParagraphIndex != originalRootParagraphIndex)
            .Where(candidate => beforeRootXml.SequenceEqual(
                afterRootXml.Where((_, index) => index != candidate.RootParagraphIndex),
                StringComparer.Ordinal))
            .OrderBy(candidate => candidate.RootParagraphIndex)
            .ToArray();
        if (afterRoots.Count != beforeRoots.Count + 1 || insertionCandidates.Length == 0)
        {
            var sharedPrefix = 0;
            while (sharedPrefix < Math.Min(beforeRootXml.Length, afterRootXml.Length) &&
                   string.Equals(
                       beforeRootXml[sharedPrefix],
                       afterRootXml[sharedPrefix],
                       StringComparison.Ordinal))
            {
                sharedPrefix++;
            }
            var firstDifference = sharedPrefix < Math.Min(beforeRootXml.Length, afterRootXml.Length)
                ? DescribeFirstDifference(
                    beforeRootXml[sharedPrefix],
                    afterRootXml[sharedPrefix])
                : "none in shared prefix";
            throw new InvalidOperationException(
                $"Box insertion did not add one removable prototype clone at the document end. " +
                $"roots={beforeRoots.Count}->{afterRoots.Count}, sharedPrefix={sharedPrefix}, " +
                $"firstDiffRoot={sharedPrefix}, firstDiff={firstDifference}, " +
                $"tables={tablesBefore}->{Count(afterDocument, "TABLE")}, " +
                $"boxes=[{string.Join(";", FindCandidates(
                    afterRoots,
                    styles,
                    requirePrototypeLineBreak: false).Select(candidate =>
                        $"{candidate.RootParagraphIndex}:{Describe(ReadLogicalLines(candidate.Parts.ContentParagraph))}"))}], " +
                $"result={insertionResult ?? "null"} ({insertionResult?.GetType().FullName ?? "null"}).");
        }
        // MoveDocEnd inserts immediately before Hancom's terminal paragraph.
        // If adjacent roots are byte-identical, removing either can reconstruct
        // the old sequence; the last matching prototype is the newly inserted one.
        var insertedIndex = insertionCandidates[^1].RootParagraphIndex;
        if (!string.Equals(
                StableBoxCloneXml(afterRoots[insertedIndex]),
                StableBoxCloneXml(afterRoots[originalRootParagraphIndex]),
                StringComparison.Ordinal))
        {
            var insertedRootXml = StableBoxCloneXml(afterRoots[insertedIndex]);
            var currentOriginalXml = StableBoxCloneXml(afterRoots[originalRootParagraphIndex]);
            throw new InvalidOperationException(
                "The inserted native box differed from its prototype before text replacement: " +
                DescribeFirstDifference(currentOriginalXml, insertedRootXml));
        }
        if (insertedIndex <= originalRootParagraphIndex)
        {
            throw new InvalidOperationException(
                "The box was not inserted after its source prototype.");
        }
        VerifyOriginal(afterRoots);
        if (Count(afterDocument, "TABLE") != tablesBefore + 1 ||
            Count(afterDocument, "PICTURE") != picturesBefore ||
            Count(afterDocument, "AUTONUM") != autoNumbersBefore)
        {
            throw new InvalidOperationException(
                "Box insertion did not add exactly one table while preserving pictures and automatic numbers.");
        }

        ReplaceCloneContent(hwp, styles, insertedIndex, lines);

        Run(hwp, "MoveDocEnd");
        return new BoxInsertion(insertedIndex);
    }

    public void VerifyOriginal(IReadOnlyList<XElement> roots)
    {
        if (originalRootParagraphIndex >= roots.Count ||
            !string.Equals(
                StableXml(roots[originalRootParagraphIndex]),
                originalRootXml,
                StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "The source box prototype changed during preview rendering.");
        }
    }

    public void VerifyRenderedRoot(
        XElement root,
        AuriPreviewStyleBindings styles,
        IReadOnlyList<string> lines)
    {
        if (!TryReadParts(root, styles, requirePrototypeLineBreak: false, out var parts))
        {
            throw new InvalidOperationException(
                "The rendered box did not retain the minimal-fixture table structure.");
        }
        var actualLines = ReadLogicalLines(parts.ContentParagraph);
        if (!actualLines.SequenceEqual(lines, StringComparer.Ordinal))
        {
            throw new InvalidOperationException(
                $"Rendered box lines differed; expected {Describe(lines)}, actual {Describe(actualLines)}.");
        }
        var renderedStructureXml = StableBoxStructureXml(root, styles);
        if (!string.Equals(
                renderedStructureXml,
                originalRootStructureXml,
                StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "Rendered box structure differed from the prototype outside its logical content: " +
                DescribeFirstDifference(originalRootStructureXml, renderedStructureXml));
        }
    }

    public static XDocument ReadDocument(dynamic hwp) =>
        XDocument.Parse((string)hwp.GetTextFile("HWPML2X", ""));

    public static IReadOnlyList<XElement> RootParagraphs(XDocument document)
    {
        var sections = document.Descendants()
            .Where(element => element.Name.LocalName == "SECTION")
            .ToArray();
        if (sections.Length != 1)
        {
            throw new InvalidOperationException(
                $"The minimal-fixture box binding requires one SECTION, found {sections.Length}.");
        }
        return sections[0].Elements()
            .Where(element => element.Name.LocalName == "P")
            .ToArray();
    }

    private void ReplaceCloneContent(
        dynamic hwp,
        AuriPreviewStyleBindings styles,
        int cloneRootParagraphIndex,
        IReadOnlyList<string> lines)
    {
        var sentinel = $"MD2HWP_BOX_{Guid.NewGuid():N}";
        XDocument beforeDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> beforeRoots = RootParagraphs(beforeDocument);
        VerifyOriginal(beforeRoots);
        if ((beforeDocument.Root?.Value ?? string.Empty).Contains(
                sentinel,
                StringComparison.Ordinal))
        {
            throw new InvalidOperationException("Generated box sentinel already existed in the document.");
        }

        MoveToRoot(hwp, cloneRootParagraphIndex);
        FindNext(hwp, PrototypeTextMarker);
        Run(hwp, "MoveParaBegin");
        Run(hwp, "MoveSelParaEnd");
        InsertText(hwp, sentinel);

        XDocument sentinelDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> sentinelRoots = RootParagraphs(sentinelDocument);
        VerifyOriginal(sentinelRoots);
        if (CountOccurrences(sentinelDocument.Root?.Value ?? string.Empty, sentinel) != 1)
        {
            throw new InvalidOperationException(
                "Box sentinel was not isolated to exactly one cloned paragraph.");
        }
        VerifyRenderedRoot(
            sentinelRoots[cloneRootParagraphIndex],
            styles,
            [sentinel]);

        MoveToRoot(hwp, cloneRootParagraphIndex);
        FindNext(hwp, sentinel);
        if (lines[0].Length == 0)
        {
            Run(hwp, "Delete");
        }
        else
        {
            InsertText(hwp, lines[0]);
        }
        for (var index = 1; index < lines.Count; index++)
        {
            Run(hwp, "BreakLine");
            if (lines[index].Length > 0)
            {
                InsertText(hwp, lines[index]);
            }
        }

        XDocument finalDocument = ReadDocument(hwp);
        IReadOnlyList<XElement> finalRoots = RootParagraphs(finalDocument);
        VerifyOriginal(finalRoots);
        if ((finalDocument.Root?.Value ?? string.Empty).Contains(
                sentinel,
                StringComparison.Ordinal))
        {
            throw new InvalidOperationException("Box sentinel remained after content replacement.");
        }
        VerifyRenderedRoot(finalRoots[cloneRootParagraphIndex], styles, lines);
    }

    private static IReadOnlyList<BoxCandidate> FindCandidates(
        IReadOnlyList<XElement> roots,
        AuriPreviewStyleBindings styles,
        bool requirePrototypeLineBreak)
    {
        var result = new List<BoxCandidate>();
        for (var index = 0; index < roots.Count; index++)
        {
            if (TryReadParts(roots[index], styles, requirePrototypeLineBreak, out var parts))
            {
                result.Add(new BoxCandidate(index, roots[index], parts));
            }
        }
        return result;
    }

    private static bool TryReadParts(
        XElement root,
        AuriPreviewStyleBindings styles,
        bool requirePrototypeLineBreak,
        out BoxParts parts)
    {
        parts = null!;
        if (ReadStyle(root) != styles.Resolve("body").Id ||
            root.Descendants().Any(element =>
                element.Name.LocalName is "PICTURE" or "AUTONUM"))
        {
            return false;
        }

        var tables = root.Descendants()
            .Where(element => element.Name.LocalName == "TABLE")
            .ToArray();
        if (tables.Length != 1)
        {
            return false;
        }
        var table = tables[0];
        if (Count(table, "ROW") != 1 ||
            Count(table, "CELL") != 1 ||
            Count(table, "PARALIST") != 2 ||
            table.Descendants().Count(element =>
                element.Name.LocalName == "POSITION" &&
                string.Equals(
                    element.Attribute("TreatAsChar")?.Value,
                    "true",
                    StringComparison.OrdinalIgnoreCase)) != 1)
        {
            return false;
        }

        var innerParagraphs = table.Descendants()
            .Where(element => element.Name.LocalName == "PARALIST")
            .SelectMany(element => element.Elements()
                .Where(child => child.Name.LocalName == "P"))
            .ToArray();
        if (innerParagraphs.Length != 2)
        {
            return false;
        }
        var content = innerParagraphs
            .Where(paragraph => ReadStyle(paragraph) == styles.Resolve("block.box").Id)
            .ToArray();
        var source = innerParagraphs
            .Where(paragraph => ReadStyle(paragraph) == styles.Resolve("figure.source").Id)
            .ToArray();
        if (content.Length != 1 || source.Length != 1 ||
            !TryReadLogicalLines(content.ElementAtOrDefault(0), out var contentLines) ||
            !TryReadLogicalLines(source.ElementAtOrDefault(0), out var sourceLines) ||
            sourceLines.Count != 1 ||
            !string.Equals(sourceLines[0], "출처: ", StringComparison.Ordinal) ||
            (requirePrototypeLineBreak &&
             (contentLines.Count < 2 ||
              CountOccurrences(string.Concat(contentLines), PrototypeTextMarker) != 1)))
        {
            return false;
        }
        parts = new BoxParts(table, content[0], source[0]);
        return true;
    }

    private static IReadOnlyList<string> ReadLogicalLines(XElement paragraph)
    {
        if (!TryReadLogicalLines(paragraph, out var lines))
        {
            throw new InvalidOperationException(
                "Expected one rendered box TEXT/CHAR hierarchy containing only text and line breaks.");
        }
        return lines;
    }

    private static bool TryReadLogicalLines(
        XElement? paragraph,
        out IReadOnlyList<string> lines)
    {
        lines = [];
        if (paragraph is null)
        {
            return false;
        }
        var textElements = paragraph.Elements()
            .Where(element => element.Name.LocalName == "TEXT")
            .ToArray();
        if (textElements.Length != 1)
        {
            return false;
        }
        var characters = textElements[0].Elements()
            .Where(element => element.Name.LocalName == "CHAR")
            .ToArray();
        if (characters.Length != 1 || textElements[0].Elements().Count() != 1)
        {
            return false;
        }
        var result = new List<string> { string.Empty };
        foreach (var node in characters[0].Nodes())
        {
            switch (node)
            {
                case XText text:
                    result[^1] += text.Value;
                    break;
                case XElement element when element.Name.LocalName == "LINEBREAK":
                    result.Add(string.Empty);
                    break;
                default:
                    return false;
            }
        }
        lines = result;
        return true;
    }

    private static void MoveToRoot(dynamic hwp, int rootParagraphIndex)
    {
        if (!(bool)hwp.SetPos(0, rootParagraphIndex, 0))
        {
            throw new InvalidOperationException(
                $"Hancom could not move to root paragraph {rootParagraphIndex}.");
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
            throw new InvalidOperationException($"Hancom could not find box text: {text}");
        }
    }

    private static void InsertText(dynamic hwp, string text)
    {
        _ = hwp.HAction.GetDefault("InsertText", hwp.HParameterSet.HInsertText.HSet);
        hwp.HParameterSet.HInsertText.Text = text;
        if (!(bool)hwp.HAction.Execute("InsertText", hwp.HParameterSet.HInsertText.HSet))
        {
            throw new InvalidOperationException("Hancom could not replace box text.");
        }
    }

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

    private static string StableXml(XElement element)
    {
        var clone = new XElement(element);
        // HWP 2020 regenerates these native instance-order attributes during
        // insertion. Box-only auto-layout fields are normalized separately so
        // non-box roots still receive the stricter comparison.
        foreach (var attribute in clone.DescendantsAndSelf()
                     .Attributes()
                     .Where(attribute => attribute.Name.LocalName is "InstId" or "ZOrder")
                     .ToArray())
        {
            attribute.Remove();
        }
        return clone.ToString(SaveOptions.DisableFormatting);
    }

    private static string StableBoxStructureXml(
        XElement root,
        AuriPreviewStyleBindings styles)
    {
        var clone = new XElement(root);
        if (!TryReadParts(clone, styles, requirePrototypeLineBreak: false, out var parts))
        {
            throw new InvalidOperationException(
                "Could not normalize the minimal-fixture box structure for comparison.");
        }
        var characters = parts.ContentParagraph.Descendants()
            .Where(element => element.Name.LocalName == "CHAR")
            .ToArray();
        if (characters.Length != 1)
        {
            throw new InvalidOperationException(
                $"Expected one box content CHAR during comparison, found {characters.Length}.");
        }
        characters[0].RemoveNodes();
        characters[0].Add("MD2HWP_BOX_CONTENT");
        return StableBoxCloneXml(clone);
    }

    private static string StableRootSequenceXml(
        XElement root,
        AuriPreviewStyleBindings styles) =>
        TryReadParts(root, styles, requirePrototypeLineBreak: false, out _)
            ? StableBoxCloneXml(root)
            : StableXml(root);

    private static string StableBoxCloneXml(XElement element)
    {
        var clone = new XElement(element);
        // These two box-layout values are recalculated from current content.
        // Width, borders, cell properties, styles, and source text remain exact.
        foreach (var attribute in clone.DescendantsAndSelf()
                     .Attributes()
                     .Where(attribute => attribute.Name.LocalName == "LastWidth")
                     .ToArray())
        {
            attribute.Remove();
        }
        foreach (var size in clone.DescendantsAndSelf()
                     .Where(element => element.Name.LocalName == "SIZE"))
        {
            size.Attribute("Height")?.Remove();
        }
        return StableXml(clone);
    }

    private static string Describe(IEnumerable<string> lines) =>
        "[" + string.Join(", ", lines.Select(line =>
            System.Text.Json.JsonSerializer.Serialize(line))) + "]";

    private static string DescribeRoot(XElement root) =>
        $"style={ReadStyle(root)},tables={root.Descendants().Count(element => element.Name.LocalName == "TABLE")}," +
        $"paragraphStyles=[{string.Join(",", root.Descendants()
            .Where(element => element.Name.LocalName == "P")
            .Select(ReadStyle))}]," +
        $"elements=[{string.Join(",", root.Descendants()
            .Select(element => element.Name.LocalName)
            .Distinct(StringComparer.Ordinal))}]";

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

    private static void Run(dynamic hwp, string action)
    {
        if (!(bool)hwp.HAction.Run(action))
        {
            throw new InvalidOperationException($"Hancom action failed: {action}");
        }
    }

    private sealed record BoxCandidate(
        int RootParagraphIndex,
        XElement Root,
        BoxParts Parts);

    private sealed record BoxParts(
        XElement Table,
        XElement ContentParagraph,
        XElement SourceParagraph);

}
