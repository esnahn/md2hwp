using System.Globalization;
using System.Text.RegularExpressions;
using System.Xml.Linq;
using System.Xml.XPath;

namespace Md2Hwp.HancomIrPreview;

// A three-row, one-column native table is a formatting prototype. Its dimensions
// and sample content do not determine the generated table's rows or columns.
internal sealed class TemplateTables(XDocument source, XElement prototype, double widthMillimeters)
{
    internal static readonly string HeaderSlot = TaggedTemplateBinding.Tag("slot:table.header");
    internal static readonly string ContentSlot = TaggedTemplateBinding.Tag("slot:table.content");
    internal static readonly string SourceSlot = TaggedTemplateBinding.Tag("slot:table.source");
    internal static readonly string CaptionSlot = TaggedTemplateBinding.Tag("slot:table.caption");
    private static readonly Regex WidthTag = new(@"^\{\{md2hwp:table\.width-mm:([^{}]+)\}\}$", RegexOptions.CultureInvariant);
    private static readonly Regex Tags = new(@"\{\{md2hwp:([^{}]+)\}\}", RegexOptions.CultureInvariant);
    private readonly HashSet<string> generatedShapeIds = [];
    private readonly HashSet<string> heightPaths = [];

    internal double WidthMillimeters => widthMillimeters;
    internal int WidthHwpUnits => checked((int)Math.Round(widthMillimeters * 7200 / 25.4));
    internal XElement Prototype => new(prototype.Descendants("TABLE").Single());
    internal IReadOnlyCollection<string> GeneratedTableInstances => generatedShapeIds;
    internal void RecalculateNumbers(XDocument document) => RecalculateTableNumbers(document);

    internal static (TemplateTables Layout, XDocument Document) Lower(XDocument source)
    {
        var document = new XDocument(source);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document).ToArray();
        int Find(string name)
        {
            var matches = roots.Select((paragraph, index) => (paragraph, index))
                .Where(item => TaggedTemplateBinding.DirectText(item.paragraph) == TaggedTemplateBinding.Tag(name)).ToArray();
            if (matches.Length != 1)
                throw new InvalidDataException($"Expected one root declaration {name}; found {matches.Length}. Regenerate with init-template or explicitly add the table prototype.");
            return matches[0].index;
        }
        var templateBegin = Find("begin:template");
        var templateEnd = Find("end:template");
        var begin = Find("begin:table");
        var end = Find("end:table");
        if (begin <= templateBegin || end >= templateEnd || end != begin + 2)
            throw new InvalidDataException("table requires exactly one root sample paragraph inside template definitions.");
        RequirePlain(roots[begin], "table boundary"); RequirePlain(roots[end], "table boundary");
        var widths = roots.Select((paragraph, index) => (paragraph, index, match: WidthTag.Match(TaggedTemplateBinding.DirectText(paragraph))))
            .Where(item => item.match.Success).ToArray();
        if (widths.Length != 1 || widths[0].index <= templateBegin || widths[0].index >= begin)
            throw new InvalidDataException("Expected one table.width-mm setting before begin:table inside template definitions.");
        RequirePlain(widths[0].paragraph, "table.width-mm");
        if (!double.TryParse(widths[0].match.Groups[1].Value, NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out var width) ||
            !double.IsFinite(width) || width <= 0 || width * 7200 / 25.4 > int.MaxValue || Math.Round(width * 7200 / 25.4) < 1)
            throw new InvalidDataException("table.width-mm must be a positive decimal width in millimeters.");

        var root = roots[begin + 1];
        var tables = root.Descendants("TABLE").ToArray();
        if (tables.Length != 1 || root.Elements().Any(element => element.Name.LocalName != "TEXT") ||
            root.Elements("TEXT").SelectMany(text => text.Elements()).Any(element =>
                element.Name.LocalName != "TABLE" && !(element.Name.LocalName == "CHAR" && !element.HasElements && element.Value.Length == 0)))
            throw new InvalidDataException("table sample requires one native table anchored alone in its root paragraph.");
        var table = tables[0];
        var rows = table.Elements("ROW").ToArray();
        if (rows.Length != 3 || rows.Any(row => row.Elements("CELL").Count() != 1) ||
            (int?)table.Attribute("RowCount") != 3 || (int?)table.Attribute("ColCount") != 1)
            throw new InvalidDataException("table sample requires exactly three rows and one column: header, content and source.");
        var accepted = new HashSet<XElement> { roots[begin], roots[end], widths[0].paragraph };
        var slots = new[] { HeaderSlot, ContentSlot, SourceSlot };
        for (var index = 0; index < rows.Length; index++)
        {
            var cell = rows[index].Elements("CELL").Single();
            if ((int?)cell.Attribute("ColSpan") is not (null or 1) || (int?)cell.Attribute("RowSpan") is not (null or 1))
                throw new InvalidDataException("table sample cells must not be merged.");
            if (cell.Elements("PARALIST").Count() != 1 ||
                cell.Elements().Any(element => element.Name.LocalName is not ("CELLMARGIN" or "PARALIST")))
                throw new InvalidDataException("Table sample cells require one native paragraph list and optional cell margins, without other controls.");
            var list = cell.Elements("PARALIST").Single();
            if (list.Elements().Any(element => element.Name.LocalName != "P"))
                throw new InvalidDataException("Table sample paragraph lists may contain only paragraphs.");
            var paragraphs = list.Elements("P").ToArray();
            if (paragraphs.Length != 1) throw new InvalidDataException("Each table sample cell requires exactly one slot paragraph.");
            RequirePlain(paragraphs[0], "table cell");
            if (Count(TaggedTemplateBinding.DirectText(paragraphs[0]), slots[index]) != 1 ||
                index < 2 && TaggedTemplateBinding.DirectText(paragraphs[0]) != slots[index])
                throw new InvalidDataException("table cells require header, content and source slots in that order; header/content slots occupy the full paragraph.");
            accepted.Add(paragraphs[0]);
        }
        RequireTransparentSource(table, rows[^1].Element("CELL")!, document);
        var shape = table.Element("SHAPEOBJECT") ?? throw new InvalidDataException("table sample has no native shape options.");
        var caption = shape.Element("CAPTION") ?? throw new InvalidDataException("table sample requires a native caption with slot:table.caption.");
        var captionLists = caption.Elements("PARALIST").ToArray();
        if (captionLists.Length != 1 || captionLists[0].Elements().Any(element => element.Name.LocalName != "P"))
            throw new InvalidDataException("Table captions require one native paragraph list containing only paragraphs.");
        var captionParagraphs = captionLists[0].Elements("P").ToArray();
        if (captionParagraphs.Length != 1 || Count(TaggedTemplateBinding.DirectText(captionParagraphs[0]), CaptionSlot) != 1)
            throw new InvalidDataException("table caption requires exactly one native paragraph with slot:table.caption.");
        var captionParagraph = captionParagraphs[0];
        if (captionParagraph.Elements().Any(element => element.Name.LocalName != "TEXT") ||
            captionParagraph.Elements("TEXT").SelectMany(text => text.Elements()).Any(element =>
                element.Name.LocalName != "AUTONUM" && !(element.Name.LocalName == "CHAR" && !element.HasElements)) ||
            captionParagraph.Descendants("AUTONUM").Count() != 1 ||
            (string?)captionParagraph.Descendants("AUTONUM").Single().Attribute("NumberType") != "Table")
            throw new InvalidDataException("table caption requires one native Table AUTONUM and no other controls.");
        if ((string?)captionParagraph.Attribute("PageBreak") == "true" || (string?)captionParagraph.Attribute("ColumnBreak") == "true")
            throw new InvalidDataException("table caption cannot carry page/column breaks.");
        var automaticNumber = captionParagraph.Descendants("AUTONUM").Single();
        if (automaticNumber.Elements().Count() != 1 || automaticNumber.Elements().Single().Name.LocalName != "AUTONUMFORMAT" ||
            automaticNumber.Elements().Single().HasElements)
            throw new InvalidDataException("Table AUTONUM requires one native automatic-number format without other controls.");
        accepted.Add(captionParagraph);
        foreach (var paragraph in document.Descendants("P"))
        foreach (Match tag in Tags.Matches(TaggedTemplateBinding.DirectText(paragraph)))
        {
            var token = tag.Groups[1].Value;
            if (!token.StartsWith("table.", StringComparison.Ordinal) &&
                !token.StartsWith("slot:table.", StringComparison.Ordinal) && token is not ("begin:table" or "end:table")) continue;
            if (!accepted.Contains(paragraph) ||
                tag.Value != HeaderSlot && tag.Value != ContentSlot && tag.Value != SourceSlot && tag.Value != CaptionSlot &&
                token is not ("begin:table" or "end:table") && !WidthTag.IsMatch(tag.Value))
                throw new InvalidDataException($"Unsupported or misplaced table declaration '{token}'.");
        }
        foreach (var paragraph in new[] { rows[2].Descendants("P").Single(), captionParagraph })
        foreach (Match tag in Tags.Matches(TaggedTemplateBinding.DirectText(paragraph)))
            if (paragraph == captionParagraph
                ? tag.Value != CaptionSlot && tag.Value != TemplateHeadingNumbers.Tag
                : tag.Value != SourceSlot)
                throw new InvalidDataException("Table source/caption samples contain an unsupported template tag.");
        foreach (var paragraph in new[] { rows[2].Descendants("P").Single(), captionParagraph })
        {
            var remaining = TaggedTemplateBinding.DirectText(paragraph).Replace(SourceSlot, "", StringComparison.Ordinal)
                .Replace(CaptionSlot, "", StringComparison.Ordinal).Replace(TemplateHeadingNumbers.Tag, "", StringComparison.Ordinal);
            if (remaining.Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
                throw new InvalidDataException("Malformed template tag in table source/caption sample.");
        }
        var layout = new TemplateTables(new XDocument(source), new XElement(root), width);
        widths[0].paragraph.Remove(); roots[begin].Remove(); root.Remove(); roots[end].Remove();
        return (layout, document);
    }

    internal XDocument Attach(XDocument rendered, IrPreviewPlan plan, int start,
        Func<PreviewTable, XDocument, XElement, int[]> columnWidths)
    {
        generatedShapeIds.Clear(); heightPaths.Clear();
        var result = new XDocument(rendered);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(result);
        var index = start;
        foreach (var operation in plan.Operations)
        {
            if (operation.Kind == "table")
            {
                if (operation.Table is not { } content || index >= roots.Count ||
                    TaggedTemplateBinding.DirectText(roots[index]) != operation.Lines.Single())
                    throw new InvalidDataException("Missing generated table placeholder or validated table content.");
                var instance = TemplateHeadingBlocks.ImportParagraph(prototype, source, result);
                var table = instance.Descendants("TABLE").Single();
                var shape = table.Element("SHAPEOBJECT")!;
                var instanceId = (string?)shape.Attribute("InstId") ?? (string?)shape.Attribute("InstID")
                    ?? throw new InvalidDataException("Table prototype requires a native shape instance ID.");
                generatedShapeIds.Add(instanceId);
                var widths = columnWidths(content, result, table);
                if (widths.Length != content.Columns.Count || widths.Any(value => value <= 0) || widths.Sum(value => (long)value) != WidthHwpUnits)
                    throw new InvalidDataException("Calculated table column widths must be positive and sum to table.width-mm.");
                var samples = table.Elements("ROW").Select(row => new XElement(row)).ToArray();
                var rows = new List<XElement>();
                void AddRow(XElement sample, IReadOnlyList<PreviewInlineContent> values, bool header)
                {
                    var row = new XElement(sample.Name, sample.Attributes());
                    for (var column = 0; column < values.Count; column++)
                    {
                        var cell = TemplateHeadingBlocks.ImportParagraph(sample.Elements("CELL").Single(), result, result);
                        cell.SetAttributeValue("ColAddr", column); cell.SetAttributeValue("RowAddr", rows.Count);
                        cell.SetAttributeValue("ColSpan", 1); cell.SetAttributeValue("RowSpan", 1);
                        cell.SetAttributeValue("Width", widths[column]); cell.SetAttributeValue("Height", 1);
                        cell.SetAttributeValue("Header", header ? "true" : "false");
                        var paragraph = cell.Elements("PARALIST").Single().Elements("P").Single();
                        FillSlot(paragraph, header ? HeaderSlot : ContentSlot, values[column], result);
                        ApplyAlignment(paragraph, content.Columns[column], result);
                        row.Add(cell);
                    }
                    rows.Add(row);
                }
                AddRow(samples[0], content.Header, true);
                foreach (var row in content.Rows) AddRow(samples[1], row, false);
                if (content.Source is { } sourceContent)
                {
                    var row = new XElement(samples[2]);
                    var cell = TemplateHeadingBlocks.ImportParagraph(row.Elements("CELL").Single(), result, result);
                    row.ReplaceNodes(cell);
                    cell.SetAttributeValue("ColAddr", 0); cell.SetAttributeValue("RowAddr", rows.Count);
                    cell.SetAttributeValue("ColSpan", widths.Length); cell.SetAttributeValue("RowSpan", 1);
                    cell.SetAttributeValue("Width", WidthHwpUnits); cell.SetAttributeValue("Height", 1);
                    cell.SetAttributeValue("Header", "false");
                    FillSlot(cell.Descendants("P").Single(), SourceSlot, sourceContent, result);
                    rows.Add(row);
                }
                table.Elements("ROW").Remove(); table.Add(rows);
                table.SetAttributeValue("RowCount", rows.Count); table.SetAttributeValue("ColCount", widths.Length);
                shape.SetAttributeValue("NumberingType", content.Caption is null ? "None" : "Table");
                var size = shape.Element("SIZE") ?? throw new InvalidDataException("Table prototype has no size settings.");
                size.SetAttributeValue("Width", WidthHwpUnits); size.SetAttributeValue("Height", rows.Count);
                size.SetAttributeValue("WidthRelTo", "Absolute"); size.SetAttributeValue("HeightRelTo", "Absolute");
                size.SetAttributeValue("Protect", "false");
                var caption = shape.Element("CAPTION")!;
                if (content.Caption is null) caption.Remove();
                else
                {
                    FillSlot(caption.Descendants("P").Single(), CaptionSlot, content.Caption, result);
                    caption.SetAttributeValue("LastWidth", (string?)caption.Attribute("Side") is "Left" or "Right"
                        ? (string?)caption.Attribute("Width") : WidthHwpUnits.ToString(CultureInfo.InvariantCulture));
                }
                var filled = TemplateHeadingNumbers.Fill([instance], operation.Heading1Number).Single();
                roots[index].ReplaceWith(filled);
            }
            index++; // NativeFigureCaption.Attach has already collapsed each figure to one root.
        }
        RecalculateTableNumbers(result);
        return result;
    }

    // Capture final paths after headings and note/reference controls are attached.
    internal void RecordLayout(XDocument document)
    {
        heightPaths.Clear();
        foreach (var table in document.Descendants("TABLE").Where(table =>
            generatedShapeIds.Contains((string?)table.Element("SHAPEOBJECT")?.Attribute("InstId") ??
                (string?)table.Element("SHAPEOBJECT")?.Attribute("InstID") ?? "")))
        foreach (var element in table.Elements("SHAPEOBJECT").Elements("SIZE")
            .Concat(table.Elements("ROW").Elements("CELL")))
            heightPaths.Add("/" + string.Join("/", element.AncestorsAndSelf().Reverse()
                .Select(node => $"{node.Name.LocalName}[{node.ElementsBeforeSelf(node.Name).Count() + 1}]")));
    }

    internal void NormalizeLayout(XDocument expected, XDocument actual)
    {
        foreach (var path in heightPaths)
        {
            var before = expected.XPathSelectElement(path)?.Attribute("Height");
            var after = actual.XPathSelectElement(path)?.Attribute("Height");
            if (before is not null && after is not null && long.TryParse(before.Value, out var oldHeight) &&
                long.TryParse(after.Value, out var newHeight) && oldHeight > 0 && newHeight > 0)
                after.Value = before.Value;
        }
    }

    internal static void RecalculateTableNumbers(XDocument document)
    {
        var next = (int?)document.Descendants("DOCSETTING").Elements("BEGINNUMBER").SingleOrDefault()?.Attribute("Table") ?? 1;
        foreach (var element in document.Descendants("SECTION").SelectMany(section => section.Descendants()))
        {
            if (element.Ancestors().Any(owner => owner.Name.LocalName is "HEADER" or "FOOTER" or "MASTERPAGE")) continue;
            if (element.Name.LocalName == "STARTNUMBER" && element.Parent?.Name.LocalName == "SECDEF" &&
                (int?)element.Attribute("Table") is > 0)
                next = (int)element.Attribute("Table")!;
            else if (element.Name.LocalName == "NEWNUM" && (string?)element.Attribute("NumberType") == "Table")
                next = (int)element.Attribute("Number")!;
            else if (element.Name.LocalName == "AUTONUM" && (string?)element.Attribute("NumberType") == "Table")
                element.SetAttributeValue("Number", next++);
        }
    }

    private static void FillSlot(XElement paragraph, string slot, PreviewInlineContent content, XDocument destination)
    {
        var marker = "MD2HWP_TABLE_SLOT_" + Guid.NewGuid().ToString("N");
        var transformed = TemplateMetadata.Transform(new XDocument(new XElement("ROOT", new XElement(paragraph))),
            slot, new Regex(Regex.Escape(slot), RegexOptions.CultureInvariant), _ => marker);
        var filled = transformed.Root!.Element("P")!;
        paragraph.ReplaceNodes(filled.Nodes().ToArray());
        var character = paragraph.Elements("TEXT").Elements("CHAR").Single(node => !node.HasElements && node.Value.Contains(marker, StringComparison.Ordinal));
        var text = character.Parent!;
        var offset = character.Value.IndexOf(marker, StringComparison.Ordinal);
        var before = new XElement(text.Name, text.Attributes());
        var after = new XElement(text.Name, text.Attributes());
        var seen = false;
        foreach (var node in text.Nodes())
        {
            if (node == character)
            {
                if (offset > 0) before.Add(new XElement("CHAR", character.Value[..offset]));
                var suffix = character.Value[(offset + marker.Length)..];
                if (suffix.Length > 0) after.Add(new XElement("CHAR", suffix));
                seen = true;
            }
            else (seen ? after : before).Add(node is XElement element ? new XElement(element) : new XText(((XText)node).Value));
        }
        var runs = RichRuns(content, text, destination);
        var replacement = new List<XElement>();
        if (before.HasElements) replacement.Add(before);
        replacement.AddRange(runs);
        if (after.HasElements) replacement.Add(after);
        text.ReplaceWith(replacement);
        Coalesce(paragraph);
    }

    private static IEnumerable<XElement> RichRuns(PreviewInlineContent content, XElement sampleRun, XDocument destination)
    {
        var id = (string?)sampleRun.Attribute("CharShape") ?? throw new InvalidDataException("Table slot has no character format.");
        var baseline = destination.Descendants("CHARSHAPE").Single(shape => (string?)shape.Attribute("Id") == id);
        var texts = new List<XElement>();
        void Add(string value, bool strong, bool emphasis, bool lineBreak)
        {
            var shape = new XElement(baseline); shape.Attribute("Id")!.Remove();
            if (strong && shape.Element("BOLD") is null) shape.Add(new XElement("BOLD"));
            if (emphasis && shape.Element("ITALIC") is null) shape.Add(new XElement("ITALIC"));
            var table = baseline.Parent!;
            var match = table.Elements("CHARSHAPE").FirstOrDefault(candidate =>
            {
                var key = new XElement(candidate); key.Attribute("Id")!.Remove(); return XNode.DeepEquals(key, shape);
            });
            if (match is null)
            {
                shape.SetAttributeValue("Id", table.Elements("CHARSHAPE").Max(candidate => (int)candidate.Attribute("Id")!) + 1);
                table.Add(shape); table.SetAttributeValue("Count", table.Elements().Count()); match = shape;
            }
            var run = new XElement(sampleRun.Name, sampleRun.Attributes());
            run.SetAttributeValue("CharShape", (string)match.Attribute("Id")!);
            run.Add(new XElement("CHAR", lineBreak ? new XElement("LINEBREAK") : new XText(value)));
            texts.Add(run);
        }
        for (var line = 0; line < content.Lines.Count; line++)
        {
            if (line > 0) Add("", false, false, true);
            foreach (var run in content.Lines[line].Runs) Add(run.Text, run.Strong, run.Emphasis, false);
        }
        if (texts.Count == 0) Add("", false, false, false);
        return texts;
    }

    private static void Coalesce(XElement paragraph)
    {
        foreach (var text in paragraph.Elements("TEXT").ToArray())
            if (text.PreviousNode is XElement previous && previous.Name == text.Name &&
                previous.Attributes().Select(attribute => (attribute.Name, attribute.Value))
                    .SequenceEqual(text.Attributes().Select(attribute => (attribute.Name, attribute.Value))))
            { previous.Add(text.Nodes().ToArray()); text.Remove(); }
        foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").ToArray())
            if (character.PreviousNode is XElement previous && previous.Name == character.Name &&
                !previous.HasAttributes && !character.HasAttributes)
            { previous.Add(character.Nodes().ToArray()); character.Remove(); }
    }

    private static void ApplyAlignment(XElement paragraph, string alignment, XDocument document)
    {
        if (alignment == "default") return;
        var id = (string?)paragraph.Attribute("ParaShape") ?? throw new InvalidDataException("Table cell slot has no paragraph shape.");
        var baseline = document.Descendants("PARASHAPE").Single(shape => (string?)shape.Attribute("Id") == id);
        var shape = new XElement(baseline); shape.Attribute("Id")!.Remove();
        shape.SetAttributeValue("Align", alignment switch { "left" => "Left", "center" => "Center", "right" => "Right", _ => throw new InvalidDataException("Invalid table alignment.") });
        var table = baseline.Parent!;
        var match = table.Elements("PARASHAPE").FirstOrDefault(candidate =>
        { var key = new XElement(candidate); key.Attribute("Id")!.Remove(); return XNode.DeepEquals(key, shape); });
        if (match is null)
        {
            shape.SetAttributeValue("Id", table.Elements("PARASHAPE").Max(candidate => (int)candidate.Attribute("Id")!) + 1);
            table.Add(shape); table.SetAttributeValue("Count", table.Elements().Count()); match = shape;
        }
        paragraph.SetAttributeValue("ParaShape", (string)match.Attribute("Id")!);
    }

    private static void RequireTransparentSource(XElement table, XElement cell, XDocument document)
    {
        var id = (string?)cell.Attribute("BorderFill") ?? (string?)table.Attribute("BorderFill");
        var border = document.Descendants("BORDERFILL").SingleOrDefault(value => (string?)value.Attribute("Id") == id)
            ?? throw new InvalidDataException("Table source cell has no border/fill definition.");
        var edges = new[] { "LEFTBORDER", "RIGHTBORDER", "TOPBORDER", "BOTTOMBORDER" };
        if (edges.Any(name => (string?)border.Element(name)?.Attribute("Type") is not (null or "None")) ||
            (string?)border.Attribute("Slash") is not (null or "0") || (string?)border.Attribute("BackSlash") is not (null or "0") ||
            (string?)border.Attribute("CenterLine") is not (null or "0") ||
            (string?)border.Attribute("Shadow") is not (null or "false" or "0") ||
            (string?)border.Attribute("ThreeD") is not (null or "false" or "0") ||
            border.Element("FILLBRUSH") is { } fill &&
                (fill.Elements().Any(element => element.Name.LocalName != "WINDOWBRUSH") ||
                    fill.Elements("WINDOWBRUSH").Any(brush => (string?)brush.Attribute("FaceColor") != "4294967295" ||
                        (string?)brush.Attribute("HatchStyle") is not (null or "None"))))
            throw new InvalidDataException("The last table sample row must be transparent: no visible borders, diagonal lines, background or hatch fill.");
    }

    private static int Count(string value, string token) => Regex.Matches(value, Regex.Escape(token), RegexOptions.CultureInvariant).Count;

    private static void RequirePlain(XElement paragraph, string role)
    {
        if (!paragraph.Elements("TEXT").Any() || paragraph.Elements().Any(element => element.Name.LocalName != "TEXT") ||
            paragraph.Nodes().OfType<XText>().Any(text => !string.IsNullOrWhiteSpace(text.Value)) ||
            paragraph.Elements("TEXT").Any(text => text.Nodes().OfType<XText>().Any(value => !string.IsNullOrWhiteSpace(value.Value)) || !text.Elements("CHAR").Any() ||
                text.Elements().Any(element => element.Name.LocalName != "CHAR" || element.HasElements)) ||
            TaggedTemplateBinding.DirectText(paragraph).Any(char.IsControl) ||
            (string?)paragraph.Attribute("PageBreak") == "true" || (string?)paragraph.Attribute("ColumnBreak") == "true")
            throw new InvalidDataException($"{role} requires a plain single-line paragraph without native controls or page/column breaks.");
    }
}
