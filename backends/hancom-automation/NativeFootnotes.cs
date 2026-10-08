using System.Xml.Linq;
using System.Text.RegularExpressions;
using System.Xml.XPath;

namespace Md2Hwp.HancomIrPreview;

// Insert native controls only after the ordinary content and heading clones have
// passed their checks. Each rendered reference receives its own native note.
internal sealed class NativeFootnotes(XDocument source, XElement sample, XElement? nativeSample = null, XElement? continuationSample = null)
{
    private static readonly Regex ReferenceMarker = new("MD2HWP_FOOTNOTE_[0-9a-f]{32}", RegexOptions.CultureInvariant | RegexOptions.NonBacktracking);
    private readonly HashSet<string> generatedNumbers = [];
    private readonly HashSet<string> generatedNoteParagraphIds = new(StringComparer.Ordinal);

    internal static NativeFootnotes Bind(XDocument document)
    {
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document).ToArray();
        var tag = TaggedTemplateBinding.Tag("footnote");
        var matches = document.Descendants("P").Where(p => TaggedTemplateBinding.DirectText(p).Trim() == tag).ToArray();
        if (matches.Length != 1)
            throw new InvalidDataException("Expected one footnote sample. Put {{md2hwp:footnote}} in a native footnote inside template definitions, or use a plain root sample paragraph. Regenerate with init-template if needed.");
        var paragraph = matches[0];
        var begin = Array.FindIndex(roots, p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("begin:template"));
        var end = Array.FindIndex(roots, p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("end:template"));
        var native = paragraph.Parent?.Parent;
        if (native?.Name.LocalName != "FOOTNOTE") native = null;
        var anchor = native is null ? paragraph : paragraph.Ancestors("P").Last();
        var index = Array.IndexOf(roots, anchor);
        if (begin < 0 || end <= begin || index <= begin || index >= end || paragraph.Elements().Any(t => t.Name.LocalName != "TEXT") ||
            !paragraph.Elements().Any() || paragraph.Elements().Any(t => !t.HasElements ||
                t.Elements().Any(c => c.Name.LocalName != "CHAR" && !(native is not null && c.Name.LocalName == "AUTONUM") ||
                    c.Name.LocalName == "CHAR" && c.HasElements &&
                        (native is null || c.Elements().Any(e => e.Name.LocalName != "TAB" || e.HasElements)))) ||
            (string?)paragraph.Attribute("PageBreak") == "true" || (string?)paragraph.Attribute("ColumnBreak") == "true")
            throw new InvalidDataException("footnote must be a plain root paragraph or one native footnote body paragraph inside template definitions, without other controls or page breaks.");
        if (native is not null)
        {
            var bodyText = string.Concat(paragraph.Elements("TEXT").Elements("CHAR").SelectMany(c => c.Nodes())
                .Select(node => node is XText text ? text.Value : "\t"));
            if (bodyText.Trim() != tag)
                throw new InvalidDataException("Native footnote body slot must not cross a tab control; only whitespace and native tabs may surround the tag.");
            if (paragraph.Ancestors().Any(e => e.Name.LocalName is "CELL" or "HEADER" or "FOOTER" or "MASTERPAGE" or "ENDNOTE") ||
                paragraph.Ancestors("FOOTNOTE").Count() != 1 ||
                native.Elements().Count() != 1 || native.Elements("PARALIST").Count() != 1 ||
                native.Element("PARALIST")!.Elements().Any(e => e.Name.LocalName != "P") ||
                native.Element("PARALIST")!.Elements("P").First() != paragraph ||
                anchor.Elements().Any(e => e.Name.LocalName != "TEXT") ||
                anchor.Elements("TEXT").SelectMany(t => t.Elements()).Any(e =>
                    e != native && !(e.Name.LocalName == "CHAR" && !e.HasElements)) ||
                TaggedTemplateBinding.DirectText(anchor).Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal) ||
                (string?)anchor.Attribute("PageBreak") == "true" || (string?)anchor.Attribute("ColumnBreak") == "true")
                throw new InvalidDataException("Native footnote sample requires one plain root anchor, a first body paragraph and at most one footnote.next sample; tables, headers and nested notes are unsupported.");
            var numbers = paragraph.Descendants("AUTONUM").ToArray();
            if (numbers.Length != 1 || (string?)numbers[0].Attribute("NumberType") != "Footnote" ||
                numbers[0].Elements().Count() != 1 || numbers[0].Element("AUTONUMFORMAT") is not { HasElements: false } ||
                numbers[0].ElementsBeforeSelf().Any(e => e.Name.LocalName == "CHAR" && e.Value.Trim().Length > 0) ||
                numbers[0].Parent!.ElementsBeforeSelf().Any(t => TaggedTemplateBinding.DirectText(new XElement("P", t)).Trim().Length > 0))
                throw new InvalidDataException("Native footnote sample requires one native Footnote AUTONUM before the body slot.");
        }
        else if (TaggedTemplateBinding.DirectText(paragraph) != tag)
            throw new InvalidDataException("Plain root footnote sample must contain only {{md2hwp:footnote}}.");
        var continuationTag = TaggedTemplateBinding.Tag("footnote.next");
        var continuations = document.Descendants("P").Where(p => TaggedTemplateBinding.DirectText(p).Contains(continuationTag, StringComparison.Ordinal)).ToArray();
        if (continuations.Length > 1) throw new InvalidDataException("Expected at most one footnote.next sample.");
        var continuation = continuations.SingleOrDefault();
        if (continuation is not null)
        {
            var rootIndex = Array.IndexOf(roots, continuation);
            var insideNative = native is not null && continuation.Parent == paragraph.Parent &&
                paragraph.ElementsAfterSelf("P").SequenceEqual(new[] { continuation });
            if (!(rootIndex > begin && rootIndex < end || insideNative) ||
                continuation.Elements().Any(e => e.Name.LocalName != "TEXT") ||
                continuation.Elements("TEXT").Any(t => !t.HasElements || t.Elements().Any(e =>
                    e.Name.LocalName is not ("CHAR" or "AUTONUM") ||
                    e.Name.LocalName == "CHAR" && e.Elements().Any(c => c.Name.LocalName != "TAB" || c.HasElements) ||
                    e.Name.LocalName == "AUTONUM" && (e.Elements().Count() != 1 || e.Element("AUTONUMFORMAT") is not { HasElements: false }))) ||
                (string?)continuation.Attribute("PageBreak") == "true" || (string?)continuation.Attribute("ColumnBreak") == "true")
                throw new InvalidDataException("footnote.next must be a root sample inside template definitions or the second paragraph of the same native footnote. Text, tabs and AUTONUM may surround the slot; other controls and page breaks are unsupported.");
            var slot = new Regex(Regex.Escape(continuationTag), RegexOptions.CultureInvariant);
            if (slot.Matches(TaggedTemplateBinding.DirectText(continuation)).Count != 1 ||
                slot.Replace(TaggedTemplateBinding.DirectText(continuation), "").Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
                throw new InvalidDataException("footnote.next requires exactly one slot and no other template tags.");
            _ = SampleAffixes(continuation, "footnote.next");
        }
        if (native is not null && native.Element("PARALIST")!.Elements("P").Any(p => p != paragraph && p != continuation))
            throw new InvalidDataException("Additional native footnote sample paragraphs must consist of one {{md2hwp:footnote.next}} paragraph.");
        return new(new XDocument(document), new XElement(paragraph), native is null ? null : new XElement(native),
            continuation is null ? null : new XElement(continuation));
    }

    // Flatten only the disposable sample anchor for the ordinary style binder.
    // The complete native note stays captured above for final insertion.
    internal XDocument LowerSample(XDocument document)
    {
        var result = new XDocument(document);
        result.Descendants("P").Where(p => TaggedTemplateBinding.DirectText(p).Contains(TaggedTemplateBinding.Tag("footnote.next"), StringComparison.Ordinal)).Remove();
        if (nativeSample is null) return result;
        var paragraph = result.Descendants("P").Single(p =>
            TaggedTemplateBinding.DirectText(p).Trim() == TaggedTemplateBinding.Tag("footnote"));
        var declaration = new XElement(paragraph);
        var shape = BodyRun(declaration).Attribute("CharShape")!.Value;
        declaration.ReplaceNodes(new XElement("TEXT", new XAttribute("CharShape", shape),
            new XElement("CHAR", TaggedTemplateBinding.Tag("footnote"))));
        paragraph.Ancestors("P").Last().ReplaceWith(declaration);
        return result;
    }

    private static XElement BodyRun(XElement paragraph, string role = "footnote")
    {
        var offset = TaggedTemplateBinding.DirectText(paragraph).IndexOf(TaggedTemplateBinding.Tag(role), StringComparison.Ordinal);
        if (offset < 0) throw new InvalidDataException("Footnote sample has no body slot.");
        foreach (var run in paragraph.Elements("TEXT"))
        foreach (var character in run.Elements("CHAR"))
        {
            if (offset < character.Value.Length) return run;
            offset -= character.Value.Length;
        }
        throw new InvalidDataException("Footnote sample has no body slot character formatting.");
    }

    private static (XElement[] Prefix, XElement[] Suffix) SampleAffixes(XElement paragraph, string role = "footnote")
    {
        var tag = TaggedTemplateBinding.Tag(role);
        var start = TaggedTemplateBinding.DirectText(paragraph).IndexOf(tag, StringComparison.Ordinal);
        var end = start + tag.Length;
        var position = 0;
        var prefix = new List<XElement>();
        var suffix = new List<XElement>();
        foreach (var run in paragraph.Elements("TEXT"))
        {
            var before = new XElement(run.Name, run.Attributes());
            var after = new XElement(run.Name, run.Attributes());
            foreach (var element in run.Elements())
            {
                if (element.Name.LocalName == "AUTONUM")
                {
                    if (position <= start) before.Add(new XElement(element));
                    else if (position >= end) after.Add(new XElement(element));
                    else throw new InvalidDataException("Footnote slot cannot cross an automatic-number control.");
                    continue;
                }
                var beforeChar = new XElement(element.Name, element.Attributes());
                var afterChar = new XElement(element.Name, element.Attributes());
                foreach (var node in element.Nodes())
                {
                    if (node is XText text)
                    {
                        var beforeLength = Math.Clamp(start - position, 0, text.Value.Length);
                        var afterOffset = Math.Clamp(end - position, 0, text.Value.Length);
                        if (beforeLength > 0) beforeChar.Add(new XText(text.Value[..beforeLength]));
                        if (afterOffset < text.Value.Length) afterChar.Add(new XText(text.Value[afterOffset..]));
                        position += text.Value.Length;
                    }
                    else if (node is XElement tab)
                    {
                        if (position <= start) beforeChar.Add(new XElement(tab));
                        else if (position >= end) afterChar.Add(new XElement(tab));
                        else throw new InvalidDataException("Native footnote body slot cannot cross a tab control.");
                    }
                }
                if (beforeChar.Nodes().Any()) before.Add(beforeChar);
                if (afterChar.Nodes().Any()) after.Add(afterChar);
            }
            if (before.HasElements) prefix.Add(before);
            if (after.HasElements) suffix.Add(after);
        }
        return (prefix.ToArray(), suffix.ToArray());
    }

    internal XDocument Attach(XDocument rendered, IrPreviewPlan plan)
    {
        generatedNumbers.Clear();
        generatedNoteParagraphIds.Clear();
        var result = new XDocument(rendered);
        var notes = FigureReferenceContract.ReadRuns(plan.Operations).Where(r => r.Footnote is not null)
            .ToDictionary(r => r.Text, r => r.Footnote!, StringComparer.Ordinal);
        if (notes.Count == 0) return result;
        var found = new HashSet<string>(StringComparer.Ordinal);
        var generated = new HashSet<XElement>();
        foreach (var paragraph in result.Descendants("P").ToArray())
        {
            if (!ReferenceMarker.Matches(TaggedTemplateBinding.DirectText(paragraph)).Any(m => notes.ContainsKey(m.Value))) continue;
            // Split mixed character payloads at line-break/tab boundaries before
            // resolving references; restore adjacent character payloads below.
            foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").Where(c => c.HasElements).ToArray())
                character.ReplaceWith(character.Nodes().Select(n => new XElement("CHAR", n is XText text ? new XText(text.Value) : new XElement((XElement)n))).ToArray());
            // A control is a boundary even when its own text contains a marker.
            var atoms = paragraph.Elements("TEXT").SelectMany(t => t.Elements())
                .SelectMany(e => e.Name.LocalName == "CHAR" && !e.HasElements
                    ? e.Value.Select((c, i) => (Element: e, Offset: i, Character: c))
                    : [(Element: e, Offset: 0, Character: '\0')]).ToArray();
            var value = new string(atoms.Select(a => a.Character).ToArray());
            var replacements = ReferenceMarker.Matches(value)
                .Where(m => notes.ContainsKey(m.Value)).Select(m => (Offset: m.Index, Marker: m.Value, Note: notes[m.Value]))
                .OrderByDescending(r => r.Offset).ToArray();
            if (replacements.Length == 0) continue;
            foreach (var replacement in replacements)
            {
                if (paragraph.Ancestors().Any(e => e.Name.LocalName is "HEADER" or "FOOTER" or "MASTERPAGE" or "CAPTION" or "FOOTNOTE" or "ENDNOTE"))
                    throw new InvalidDataException("Footnote references cannot be placed in native captions, headers, footers, master pages or other notes.");
                var first = atoms[replacement.Offset];
                var last = atoms[replacement.Offset + replacement.Marker.Length - 1];
                var control = Create(replacement.Note, result);
                // Track the first body paragraph through later reference TEXT
                // splitting. Native paragraph identities survive XML cloning;
                // paths below are recaptured only after all attachments finish.
                var firstParagraph = control.Elements("PARALIST").Single().Elements("P").First();
                var used = result.Descendants().Attributes().Where(attribute =>
                    attribute.Name.LocalName is "InstId" or "InstID").Select(attribute => attribute.Value)
                    .Concat(control.Descendants().Attributes().Where(attribute =>
                        attribute.Name.LocalName is "InstId" or "InstID").Select(attribute => attribute.Value))
                    .Concat(generatedNoteParagraphIds).ToHashSet(StringComparer.Ordinal);
                string identity;
                do { identity = BitConverter.ToUInt32(Guid.NewGuid().ToByteArray(), 0).ToString(System.Globalization.CultureInfo.InvariantCulture); }
                while (!used.Add(identity));
                var identityAttribute = firstParagraph.Attribute("InstId") ?? firstParagraph.Attribute("InstID");
                if (identityAttribute is null) firstParagraph.SetAttributeValue("InstId", identity);
                else identityAttribute.Value = identity;
                generatedNoteParagraphIds.Add(identity);
                generated.Add(control); found.Add(replacement.Marker);
                var prefix = first.Element.Value[..first.Offset];
                var suffix = last.Element.Value[(last.Offset + 1)..];
                if (first.Element == last.Element)
                {
                    first.Element.Value = prefix;
                    first.Element.AddAfterSelf(control, new XElement("CHAR", suffix));
                }
                else
                {
                    var consumed = atoms.Skip(replacement.Offset).Take(replacement.Marker.Length).Select(a => a.Element).Distinct().ToArray();
                    first.Element.Value = prefix;
                    first.Element.AddAfterSelf(control);
                    foreach (var element in consumed.Skip(1))
                        if (element == last.Element) element.Value = suffix; else element.Remove();
                }
            }
            paragraph.Elements("TEXT").Elements("CHAR").Where(c => !c.HasElements && c.Value.Length == 0).Remove();
            paragraph.Elements("TEXT").Where(t => !t.HasElements).Remove();
            // Hancom combines adjacent character runs with identical formats
            // after replacing a marker-only run with a native control.
            foreach (var text in paragraph.Elements("TEXT").ToArray())
            {
                if (text.PreviousNode is not XElement previous || previous.Name != text.Name ||
                    !previous.Attributes().OrderBy(a => a.Name.ToString()).Select(a => (a.Name, a.Value))
                        .SequenceEqual(text.Attributes().OrderBy(a => a.Name.ToString()).Select(a => (a.Name, a.Value)))) continue;
                previous.Add(text.Nodes().ToArray()); text.Remove();
            }
            foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").ToArray())
                if (character.PreviousNode is XElement previous && previous.Name.LocalName == "CHAR" &&
                    !previous.HasAttributes && !character.HasAttributes)
                { previous.Add(character.Nodes().ToArray()); character.Remove(); }
        }
        if (!found.SetEquals(notes.Keys)) throw new InvalidOperationException("A generated footnote reference was lost before native insertion.");
        // Continuous/section counters are known without page layout. Page-based
        // restarts are calculated by Hancom and verified separately below.
        var next = (int?)result.Descendants("DOCSETTING").Elements("BEGINNUMBER").SingleOrDefault()?.Attribute("Footnote") ?? 1;
        foreach (var element in result.Descendants("SECTION").Single().Descendants())
        {
            if (element.Ancestors().Any(e => e.Name.LocalName is "HEADER" or "FOOTER" or "MASTERPAGE")) continue;
            if (element.Name.LocalName == "NOTENUMBERING" && element.Parent?.Name.LocalName == "FOOTNOTESHAPE" && (string?)element.Attribute("Type") == "OnSection")
                next = (int?)element.Attribute("NewNumber") ?? 1;
            else if (element.Name.LocalName == "NEWNUM" && (string?)element.Attribute("NumberType") == "Footnote")
                next = (int)element.Attribute("Number")!;
            else if (element.Name.LocalName == "AUTONUM" && (string?)element.Attribute("NumberType") == "Footnote")
            {
                var note = element.Ancestors("FOOTNOTE").FirstOrDefault();
                if (note is not null && element.Ancestors("P").First() != note.Element("PARALIST")!.Elements("P").First()) continue;
                if (element.Ancestors("FOOTNOTE").Any(generated.Contains)) element.SetAttributeValue("Number", next);
                next++;
            }
        }
        RecordLayout(result);
        return result;
    }

    internal void RecordLayout(XDocument document)
    {
        generatedNumbers.Clear();
        var found = new HashSet<string>(StringComparer.Ordinal);
        foreach (var control in document.Descendants("FOOTNOTE"))
        {
            var firstParagraph = control.Elements("PARALIST").SingleOrDefault()?.Elements("P").FirstOrDefault();
            var identity = (string?)firstParagraph?.Attribute("InstId") ?? (string?)firstParagraph?.Attribute("InstID");
            if (identity is null || !generatedNoteParagraphIds.Contains(identity)) continue;
            if (!found.Add(identity))
                throw new InvalidOperationException("A generated footnote paragraph has an ambiguous native identity.");
            var numbers = firstParagraph!.Descendants("AUTONUM").Where(number =>
                (string?)number.Attribute("NumberType") == "Footnote").ToArray();
            if (numbers.Length != 1)
                throw new InvalidOperationException("A generated footnote lost its unique native automatic-number control.");
            generatedNumbers.Add(Path(numbers[0]));
        }
        if (!found.SetEquals(generatedNoteParagraphIds))
            throw new InvalidOperationException("A generated footnote was lost before final layout capture.");
    }

    internal void NormalizeNumbers(XDocument expected, XDocument actual)
    {
        // Only page restart counters require native layout. Preserve exact values
        // for continuous counters and every unrelated pre-existing control.
        var numbering = expected.Descendants("FOOTNOTESHAPE").Elements("NOTENUMBERING").SingleOrDefault();
        if ((string?)numbering?.Attribute("Type") != "OnPage") return;
        foreach (var path in generatedNumbers)
        {
            var left = expected.XPathSelectElement(path)?.Attribute("Number");
            var right = actual.XPathSelectElement(path)?.Attribute("Number");
            if (left is not null && right is not null && int.TryParse(right.Value, out var number) && number > 0)
                right.Value = left.Value;
        }
    }

    private XElement Create(PreviewFootnote note, XDocument destination)
    {
        var format = destination.Descendants("FOOTNOTESHAPE").Elements("AUTONUMFORMAT").SingleOrDefault()
            ?? throw new InvalidDataException("The template section has no native footnote numbering format.");
        if (nativeSample is not null)
        {
            var control = TemplateHeadingBlocks.ImportParagraph(nativeSample, source, destination);
            var list = control.Element("PARALIST")!;
            var affixes = SampleAffixes(list.Element("P")!);
            list.ReplaceNodes(note.Paragraphs.Select((content, index) =>
            {
                var paragraph = Fill(content, destination, null, index > 0);
                if (index == 0)
                {
                    paragraph.AddFirst(affixes.Prefix);
                    paragraph.Add(affixes.Suffix);
                }
                foreach (var run in paragraph.Elements("TEXT").ToArray())
                    if (run.PreviousNode is XElement previous && previous.Name == run.Name &&
                        previous.Attributes().Select(a => (a.Name, a.Value)).SequenceEqual(run.Attributes().Select(a => (a.Name, a.Value))))
                    { previous.Add(run.Nodes().ToArray()); run.Remove(); }
                foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").ToArray())
                    if (character.PreviousNode is XElement previous && previous.Name == character.Name && !previous.HasAttributes && !character.HasAttributes)
                    { previous.Add(character.Nodes().ToArray()); character.Remove(); }
                return paragraph;
            }).ToArray());
            return control;
        }
        var paragraphs = note.Paragraphs.Select((content, index) => Fill(content, destination, index == 0 ? format : null, index > 0)).ToArray();
        return new XElement("FOOTNOTE", new XElement("PARALIST",
            new XAttribute("LineWrap", "Break"), new XAttribute("LinkListID", "0"), new XAttribute("LinkListIDNext", "0"),
            new XAttribute("TextDirection", "0"), new XAttribute("VertAlign", "Top"), paragraphs));
    }

    private XElement Fill(PreviewInlineContent content, XDocument destination, XElement? numberFormat, bool continuation)
    {
        var useContinuation = continuation && continuationSample is not null;
        var paragraph = TemplateHeadingBlocks.ImportParagraph(useContinuation ? continuationSample! : sample, source, destination);
        var affixes = useContinuation ? SampleAffixes(paragraph, "footnote.next") : (Prefix: Array.Empty<XElement>(), Suffix: Array.Empty<XElement>());
        paragraph.Attribute("PageBreak")?.Remove(); paragraph.Attribute("ColumnBreak")?.Remove();
        var id = (string)BodyRun(paragraph, useContinuation ? "footnote.next" : "footnote").Attribute("CharShape")!;
        var baseline = destination.Descendants("CHARSHAPE").Single(c => (string?)c.Attribute("Id") == id);
        var texts = new List<XElement>();
        if (numberFormat is not null)
            texts.Add(new XElement("TEXT", new XAttribute("CharShape", id),
                new XElement("AUTONUM", new XAttribute("Number", 1), new XAttribute("NumberType", "Footnote"), new XElement(numberFormat)),
                new XElement("CHAR", " ")));
        void Add(string text, bool strong, bool emphasis, bool lineBreak = false)
        {
            var shape = new XElement(baseline); shape.Attribute("Id")!.Remove();
            if (strong && shape.Element("BOLD") is null) shape.Add(new XElement("BOLD"));
            if (emphasis && shape.Element("ITALIC") is null) shape.Add(new XElement("ITALIC"));
            var table = baseline.Parent!;
            var match = table.Elements("CHARSHAPE").FirstOrDefault(c => { var key = new XElement(c); key.Attribute("Id")!.Remove(); return XNode.DeepEquals(key, shape); });
            if (match is null)
            {
                shape.SetAttributeValue("Id", table.Elements("CHARSHAPE").Max(c => (int)c.Attribute("Id")!) + 1);
                table.Add(shape); table.SetAttributeValue("Count", table.Elements().Count()); match = shape;
            }
            var shapeId = (string)match.Attribute("Id")!;
            var run = texts.LastOrDefault();
            if (run is null || (string?)run.Attribute("CharShape") != shapeId)
            { run = new XElement("TEXT", new XAttribute("CharShape", shapeId), new XElement("CHAR")); texts.Add(run); }
            if (lineBreak) run.Element("CHAR")!.Add(new XElement("LINEBREAK")); else run.Element("CHAR")!.Add(new XText(text));
        }
        for (var i = 0; i < content.Lines.Count; i++)
        {
            if (i > 0) Add("", false, false, true);
            foreach (var run in content.Lines[i].Runs)
            {
                if (run.Footnote is not null) throw new InvalidDataException("Nested footnotes are not supported.");
                Add(run.Text, run.Strong, run.Emphasis);
            }
        }
        if (texts.Count == 0) Add("", false, false);
        paragraph.ReplaceNodes(affixes.Prefix, texts, affixes.Suffix);
        return paragraph;
    }

    private static string Path(XElement element) => "/" + string.Join("/", element.AncestorsAndSelf().Reverse()
        .Select(e => $"{e.Name.LocalName}[{e.ElementsBeforeSelf(e.Name).Count() + 1}]"));
}
