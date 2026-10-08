using System.Xml.Linq;
using System.Xml.XPath;

namespace Md2Hwp.HancomIrPreview;

// Native cloning and its checks use the existing one-paragraph intermediate.
// Before publication, expand raw IR lines into template-owned paragraphs.
internal sealed class TemplateBoxParagraphs(XDocument source, XElement contentSample, XElement? titleSample)
{
    private readonly HashSet<string> generatedShapeIds = [];
    private readonly HashSet<string> heightPaths = [];

    internal const string TitleTag = "{{md2hwp:slot:code.title}}";
    private const string TitlePrefix = "제목:";

    internal static (TemplateBoxParagraphs Layout, XDocument Document) Lower(XDocument source)
    {
        var document = new XDocument(source);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document).ToArray();
        int Find(string token)
        {
            var matches = roots.Select((p, i) => (p, i))
                .Where(x => TaggedTemplateBinding.DirectText(x.p) == TaggedTemplateBinding.Tag(token)).ToArray();
            if (matches.Length != 1) throw new InvalidDataException($"Expected one {token} declaration.");
            return matches[0].i;
        }
        var begin = Find("begin:code");
        var end = Find("end:code");
        if (end != begin + 2)
            throw new InvalidDataException("Box range must contain exactly one root paragraph.");
        var tables = roots[begin + 1].Descendants("TABLE").ToArray();
        if (tables.Length != 1 || tables[0].Descendants("ROW").Count() != 1 ||
            tables[0].Descendants("CELL").Count() != 1)
            throw new InvalidDataException("Box requires one single-cell table.");
        var cell = tables[0].Descendants("CELL").Single();
        var lists = cell.Elements("PARALIST").ToArray();
        if (lists.Length != 1) throw new InvalidDataException("Box cell requires one paragraph list.");
        var paragraphs = lists[0].Elements().ToArray();
        var content = paragraphs.Where(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("slot:code.content")).ToArray();
        var titles = document.Descendants("P").Where(p =>
            TaggedTemplateBinding.DirectText(p).Contains(TitleTag, StringComparison.Ordinal)).ToArray();
        if (content.Length != 1 || !PlainSlot(content[0]))
            throw new InvalidDataException("Box requires one plain content slot paragraph inside its cell.");
        if (titles.Length > 1 || titles.Length == 1 &&
            (titles[0].Parent != lists[0] || TaggedTemplateBinding.DirectText(titles[0]) != TitleTag || !PlainSlot(titles[0])))
            throw new InvalidDataException("slot:code.title must be one plain standalone paragraph in the box cell.");
        var title = titles.SingleOrDefault();
        if (paragraphs.Length != (title is null ? 1 : 2) ||
            paragraphs[^1] != content[0] || title is not null && paragraphs[0] != title)
            throw new InvalidDataException("Box cell requires an optional title slot followed by one content slot, with no extra paragraphs.");
        var layout = new TemplateBoxParagraphs(new XDocument(source), new XElement(content[0]),
            title is null ? null : new XElement(title));
        title?.Remove();
        return (layout, document);
    }

    private static bool PlainSlot(XElement paragraph) =>
        paragraph.Name.LocalName == "P" && paragraph.Elements().Any() &&
        paragraph.Elements().All(t => t.Name.LocalName == "TEXT" && t.Elements().Any() &&
            t.Elements().All(c => c.Name.LocalName == "CHAR" && !c.HasElements));

    internal XDocument Attach(XDocument rendered, IrPreviewPlan plan, int start)
    {
        generatedShapeIds.Clear();
        heightPaths.Clear();
        var result = new XDocument(rendered);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(result);
        var index = start;
        foreach (var operation in plan.Operations)
        {
            if (operation.Kind == "code")
            {
                if (index >= roots.Count) throw new InvalidDataException("Missing generated box paragraph.");
                var table = roots[index].Descendants("TABLE").Single();
                var shapeId = (string?)table.Element("SHAPEOBJECT")?.Attribute("InstId");
                if (shapeId is not null) generatedShapeIds.Add(shapeId);
                var list = table.Descendants("CELL").Single().Elements("PARALIST").Single();
                var paragraphs = new List<XElement>();
                var firstBodyLine = 0;
                if (titleSample is not null && operation.Lines[0].StartsWith(TitlePrefix, StringComparison.Ordinal))
                {
                    var title = operation.Lines[0][TitlePrefix.Length..];
                    if (title.StartsWith(' ')) title = title[1..];
                    if (string.IsNullOrWhiteSpace(title))
                        throw new InvalidDataException("Box 제목: requires a nonempty title.");
                    paragraphs.Add(Fill(titleSample, title, result));
                    firstBodyLine = 1;
                    while (firstBodyLine < operation.Lines.Count && string.IsNullOrWhiteSpace(operation.Lines[firstBodyLine]))
                        firstBodyLine++;
                }
                foreach (var line in operation.Lines.Skip(firstBodyLine))
                    paragraphs.Add(Fill(contentSample, line, result));
                list.ReplaceNodes(paragraphs);
            }
            index += operation.Kind == "figure" ? (operation.Lines[2].Length > 0 ? 3 : 2) : 1;
        }
        return result;
    }

    // Capture paths after heading blocks have changed root positions.
    internal void RecordLayout(XDocument attached)
    {
        heightPaths.Clear();
        foreach (var table in attached.Descendants("TABLE").Where(t =>
            generatedShapeIds.Contains((string?)t.Element("SHAPEOBJECT")?.Attribute("InstId") ?? "")))
        foreach (var element in table.Elements("SHAPEOBJECT").Elements("SIZE")
                     .Concat(table.Elements("ROW").Elements("CELL")))
            heightPaths.Add("/" + string.Join("/", element.AncestorsAndSelf().Reverse()
                .Select(e => $"{e.Name.LocalName}[{e.ElementsBeforeSelf(e.Name).Count() + 1}]")));
    }

    // Table/cell heights reflow when line breaks become template paragraphs.
    // Normalize only generated boxes; width, borders and formatting stay exact.
    internal void NormalizeLayout(XDocument expected, XDocument actual)
    {
        foreach (var path in heightPaths)
        {
            var left = expected.XPathSelectElement(path)?.Attribute("Height");
            var right = actual.XPathSelectElement(path)?.Attribute("Height");
            if (left is null || right is null || left.Value == right.Value) continue;
            if (long.TryParse(left.Value, out var oldHeight) && oldHeight > 0 &&
                long.TryParse(right.Value, out var newHeight) && newHeight > 0)
                right.Value = left.Value;
        }
    }

    private XElement Fill(XElement sample, string value, XDocument destination)
    {
        var paragraph = TemplateHeadingBlocks.ImportParagraph(sample, source, destination);
        var text = paragraph.Elements("TEXT").First(t => t.Elements("CHAR").Any(c => c.Value.Length > 0));
        // Slots occupy the whole paragraph, so the first tag character owns its format.
        var run = new XElement(text.Name, text.Attributes(), new XElement("CHAR", value));
        paragraph.ReplaceNodes(run);
        return paragraph;
    }
}
