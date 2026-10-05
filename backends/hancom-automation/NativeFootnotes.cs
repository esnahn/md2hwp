using System.Xml.Linq;
using System.Text.RegularExpressions;
using System.Xml.XPath;

namespace Md2Hwp.HancomIrPreview;

// Insert native controls only after the ordinary content and heading clones have
// passed their checks. Each rendered reference receives its own native note.
internal sealed class NativeFootnotes(XDocument source, XElement sample)
{
    private static readonly Regex ReferenceMarker = new("MD2HWP_FOOTNOTE_[0-9a-f]{32}", RegexOptions.CultureInvariant | RegexOptions.NonBacktracking);
    private readonly HashSet<string> generatedNumbers = [];
    private readonly HashSet<string> generatedNoteParagraphIds = new(StringComparer.Ordinal);

    internal static NativeFootnotes Bind(XDocument document)
    {
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document).ToArray();
        var matches = roots.Where(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("footnote")).ToArray();
        if (matches.Length != 1)
            throw new InvalidDataException("Expected one root declaration footnote. Add {{md2hwp:footnote}} inside the template definitions or regenerate with init-template.");
        var paragraph = matches[0];
        var begin = Array.FindIndex(roots, p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("begin:template"));
        var end = Array.FindIndex(roots, p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("end:template"));
        var index = Array.IndexOf(roots, paragraph);
        if (index <= begin || index >= end || paragraph.Elements().Any(t => t.Name.LocalName != "TEXT") ||
            !paragraph.Elements().Any() || paragraph.Elements().Any(t => !t.HasElements ||
                t.Elements().Any(c => c.Name.LocalName != "CHAR" || c.HasElements)) ||
            (string?)paragraph.Attribute("PageBreak") == "true" || (string?)paragraph.Attribute("ColumnBreak") == "true")
            throw new InvalidDataException("footnote must be one plain root sample paragraph inside the template definitions, without controls or page breaks.");
        return new(new XDocument(document), new XElement(paragraph));
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
                if (paragraph.Ancestors().Any(e => e.Name.LocalName is "HEADER" or "FOOTER" or "MASTERPAGE" or "FOOTNOTE" or "ENDNOTE"))
                    throw new InvalidDataException("Footnote references cannot be placed in headers, footers, master pages or other notes.");
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
            var numbers = control.Descendants("AUTONUM").Where(number =>
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
        var paragraphs = note.Paragraphs.Select((content, index) => Fill(content, destination, index == 0 ? format : null)).ToArray();
        return new XElement("FOOTNOTE", new XElement("PARALIST",
            new XAttribute("LineWrap", "Break"), new XAttribute("LinkListID", "0"), new XAttribute("LinkListIDNext", "0"),
            new XAttribute("TextDirection", "0"), new XAttribute("VertAlign", "Top"), paragraphs));
    }

    private XElement Fill(PreviewInlineContent content, XDocument destination, XElement? numberFormat)
    {
        var paragraph = TemplateHeadingBlocks.ImportParagraph(sample, source, destination);
        paragraph.Attribute("PageBreak")?.Remove(); paragraph.Attribute("ColumnBreak")?.Remove();
        var id = (string)paragraph.Elements("TEXT").First(t => t.Elements("CHAR").Any(c => c.Value.Length > 0)).Attribute("CharShape")!;
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
        paragraph.ReplaceNodes(texts);
        return paragraph;
    }

    private static string Path(XElement element) => "/" + string.Join("/", element.AncestorsAndSelf().Reverse()
        .Select(e => $"{e.Name.LocalName}[{e.ElementsBeforeSelf(e.Name).Count() + 1}]"));
}
