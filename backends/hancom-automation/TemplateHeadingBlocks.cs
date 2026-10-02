using System.Xml.Linq;
using System.Xml.XPath;

namespace Md2Hwp.HancomIrPreview;

// Optional, template-only presentation of an existing IR heading.
internal sealed class TemplateHeadingBlocks(XDocument source, Dictionary<string, XElement[]> samples)
{
    private readonly HashSet<string> titleHeightElements = [];
    private readonly HashSet<string> pageBreakAfter = [];

    // Only containers enclosing a replaced title may reflow. All other geometry
    // and all content/formatting remain part of the strict structural comparison.
    internal void NormalizeTitleLayout(XDocument expected, XDocument actual, bool report)
    {
        var changes = 0;
        foreach (var path in titleHeightElements)
        {
            var left = expected.XPathSelectElement(path)?.Attribute("Height");
            var right = actual.XPathSelectElement(path)?.Attribute("Height");
            if (left is null || right is null || left.Value == right.Value) continue;
            if (!long.TryParse(left.Value, out var oldHeight) || !long.TryParse(right.Value, out var newHeight) || oldHeight <= 0 || newHeight <= 0) continue;
            right.Value = left.Value;
            changes++;
        }
        if (report && changes > 0)
            Console.Error.WriteLine($"md2hwp-backend: heading layout notice: {changes} table/cell heights changed after title insertion; check wrapping and page placement in the output.");
    }

    internal static (TemplateHeadingBlocks Layout, XDocument Document) Lower(XDocument source)
    {
        var document = new XDocument(source);
        var samples = new Dictionary<string, XElement[]>(StringComparer.Ordinal);
        var pageBreakAfter = new HashSet<string>();
        for (var level = 1; level <= 6; level++)
        {
            var role = $"heading{level}";
            var roots = AuriMinimalBoxPrototype.RootParagraphs(document).ToArray();
            var begin = roots.Where(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("begin:" + role)).ToArray();
            var end = roots.Where(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("end:" + role)).ToArray();
            if (begin.Length == 0 && end.Length == 0) continue;
            if (begin.Length != 1 || end.Length != 1 || roots.Any(p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag(role)))
                throw new InvalidDataException($"{role} requires either one paragraph declaration or one block, not both.");
            int first = Array.IndexOf(roots, begin[0]), last = Array.IndexOf(roots, end[0]);
            int templateBegin = Array.FindIndex(roots, p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("begin:template"));
            int templateEnd = Array.FindIndex(roots, p => TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("end:template"));
            if (first <= templateBegin || last >= templateEnd || last <= first + 1 || last - first > 65)
                throw new InvalidDataException($"{role} block requires 1–64 paragraphs inside the template declarations.");
            var range = roots.Skip(first + 1).Take(last - first - 1).ToArray();
            foreach (var boundary in new[] { begin[0], end[0] })
                if (boundary.Elements().Any(e => e.Name.LocalName != "TEXT") ||
                    boundary.Elements().SelectMany(e => e.Elements()).Any(e => e.Name.LocalName != "CHAR" || e.HasElements))
                    throw new InvalidDataException($"{role} boundaries must be plain standalone paragraphs.");
            // Section boundaries change the document container, rather than a
            // paragraph range. Do not silently flatten them into this block.
            if (range.SelectMany(p => p.Descendants()).Any(e => e.Name.LocalName is "SECDEF" or "COLDEF"))
                throw new InvalidDataException($"{role} supports page breaks, but not section/column definitions inside its block.");
            if (range.SelectMany(p => p.Descendants()).Any(e => e.Name.LocalName is "PICTURE" or "OLE" or "VIDEO"))
                throw new InvalidDataException($"{role} embedded pictures, OLE and video are not supported; tables, drawing text and grouped shapes are supported.");
            var checkedRange = range.Select(p => new XElement(p)).ToArray();
            TemplateOnceRanges.Apply(checkedRange, first: true);
            var slots = TemplateHeadingEach.Validate(checkedRange, level);
            var prototype = range.Select(p => new XElement(p)).ToArray();
            if ((string?)begin[0].Attribute("PageBreak") == "true") SetPageBreak(prototype[0]);
            if ((string?)end[0].Attribute("PageBreak") == "true") pageBreakAfter.Add(role);
            samples.Add(role, prototype);
            var declaration = new XElement(slots[0]);
            declaration.ReplaceNodes(new XElement("TEXT", new XAttribute("CharShape", (string?)slots[0].Element("TEXT")?.Attribute("CharShape") ?? "0"),
                new XElement("CHAR", TaggedTemplateBinding.Tag(role))));
            begin[0].ReplaceWith(declaration);
            foreach (var paragraph in range) paragraph.Remove();
            end[0].Remove();
        }
        var layout = new TemplateHeadingBlocks(new XDocument(source), samples);
        layout.pageBreakAfter.UnionWith(pageBreakAfter);
        return (layout, document);
    }

    // Native figure captions are already attached: each operation now owns one root.
    internal XDocument Attach(XDocument rendered, IrPreviewPlan plan, int start)
    {
        titleHeightElements.Clear();
        var reflow = new HashSet<XElement>();
        var usedRoles = new HashSet<string>(StringComparer.Ordinal);
        var result = new XDocument(rendered);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(result).ToArray();
        for (var index = 0; index < plan.Operations.Count; index++)
        {
            var operation = plan.Operations[index];
            if (operation.Kind != "text" || operation.ParagraphStyle is null || !samples.TryGetValue(operation.ParagraphStyle, out var sample)) continue;
            var generated = roots[start + index];
            var instance = sample.Select(p => new XElement(p)).ToArray();
            TemplateOnceRanges.Apply(instance, usedRoles.Add(operation.ParagraphStyle));
            var block = instance.Select(p => ImportParagraph(p, source, result)).ToArray();
            if ((string?)generated.Attribute("PageBreak") == "true") SetPageBreak(block[0]);
            block = TemplateHeadingEach.Expand(TemplateHeadingNumbers.Fill(block, operation.Heading1Number), int.Parse(operation.ParagraphStyle[7..]), index, plan,
                (target, title, ordinal) =>
                {
                    target.AddAnnotation(title);
                    FillTitle(target, title, result);
                    if (ordinal > 0) SetRepeatedNumber(target, ordinal, result);
                });
            foreach (var target in block.SelectMany(p => p.DescendantsAndSelf("P")).Where(p => p.Annotation<PreviewOperation>() is not null))
            foreach (var table in target.Ancestors("TABLE"))
            {
                foreach (var size in table.Elements("SHAPEOBJECT").Elements("SIZE")) reflow.Add(size);
                foreach (var cell in table.Elements("ROW").Elements("CELL")) reflow.Add(cell);
            }
            // Repetition duplicates object identities; assign fresh ids to all clones.
            var identities = result.Descendants().Attributes().Where(a => a.Name.LocalName is "InstId" or "InstID").Select(a => a.Value).ToHashSet();
            foreach (var attribute in block.SelectMany(p => p.DescendantsAndSelf()).Attributes().Where(a => a.Name.LocalName is "InstId" or "InstID"))
            {
                string id;
                do { id = BitConverter.ToUInt32(Guid.NewGuid().ToByteArray(), 0).ToString(System.Globalization.CultureInfo.InvariantCulture); } while (!identities.Add(id));
                attribute.Value = id;
            }
            generated.ReplaceWith(block);
            if (pageBreakAfter.Contains(operation.ParagraphStyle) && start + index + 1 < roots.Length)
                SetPageBreak(roots[start + index + 1]);
        }
        if (samples.Values.SelectMany(p => p).SelectMany(p => p.Descendants("NEWNUM"))
            .Any(e => (string?)e.Attribute("NumberType") == "Figure")) RecalculateFigureNumbers(result);
        foreach (var element in reflow)
            titleHeightElements.Add("/" + string.Join("/", element.AncestorsAndSelf().Reverse()
                .Select(e => $"{e.Name.LocalName}[{e.ElementsBeforeSelf(e.Name).Count() + 1}]")));
        return result;
    }

    private static void SetRepeatedNumber(XElement paragraph, int number, XDocument document)
    {
        var shape = document.Descendants("PARASHAPE").Single(e => (string?)e.Attribute("Id") == (string?)paragraph.Attribute("ParaShape"));
        if ((string?)shape.Attribute("HeadingType") != "Number") return;
        var numbering = new XElement(document.Descendants("NUMBERING").Single(e => (string?)e.Attribute("Id") == (string?)shape.Attribute("Heading")));
        numbering.SetAttributeValue("Start", number);
        var level = (int?)shape.Attribute("Level") ?? 0;
        numbering.Elements("PARAHEAD").Single(e => (int?)e.Attribute("Level") == level + 1).SetAttributeValue("Start", number);
        string Add(XElement definition, XElement table)
        {
            definition.SetAttributeValue("Id", table.Elements(definition.Name).Max(e => (int)e.Attribute("Id")!) + 1);
            table.Add(definition); table.SetAttributeValue("Count", table.Elements().Count());
            return (string)definition.Attribute("Id")!;
        }
        var numberId = Add(numbering, document.Descendants("NUMBERING").First().Parent!);
        var copy = new XElement(shape); copy.SetAttributeValue("Heading", numberId);
        paragraph.SetAttributeValue("ParaShape", Add(copy, shape.Parent!));
    }

    private static void SetPageBreak(XElement paragraph)
    {
        paragraph.SetAttributeValue("PageBreak", "true");
        if (paragraph.Attribute("ColumnBreak") is null) paragraph.SetAttributeValue("ColumnBreak", "false");
    }

    private static void FillTitle(XElement target, PreviewOperation operation, XDocument document)
    {
        var id = (string?)target.Elements("TEXT").FirstOrDefault(t => t.Elements("CHAR").Any(c => c.Value.Length > 0))?.Attribute("CharShape")
            ?? throw new InvalidDataException("Heading slot has no character format.");
        var baseline = document.Descendants("CHARSHAPE").Single(c => (string?)c.Attribute("Id") == id);
        var table = baseline.Parent!;
        var runs = operation.FormattedLines?.Single() ?? [new PreviewTextRun(operation.Lines.Single(), false, false)];
        var texts = new List<XElement>();
        foreach (var run in runs)
        {
            var shape = new XElement(baseline);
            shape.Attribute("Id")!.Remove();
            if (run.Strong && shape.Element("BOLD") is null) shape.Add(new XElement("BOLD"));
            if (run.Emphasis && shape.Element("ITALIC") is null) shape.Add(new XElement("ITALIC"));
            var match = table.Elements("CHARSHAPE").FirstOrDefault(c =>
            {
                var key = new XElement(c); key.Attribute("Id")?.Remove();
                return XNode.DeepEquals(key, shape);
            });
            if (match is null)
            {
                shape.SetAttributeValue("Id", table.Elements("CHARSHAPE").Max(c => (int)c.Attribute("Id")!) + 1);
                table.Add(shape); table.SetAttributeValue("Count", table.Elements().Count()); match = shape;
            }
            texts.Add(new XElement("TEXT", new XAttribute("CharShape", (string)match.Attribute("Id")!), new XElement("CHAR", run.Text)));
        }
        target.ReplaceNodes(texts);
    }

    // NEWNUM is retained verbatim. Update only the calculated Figure AUTONUM
    // display value to agree with the native counter at its new position.
    internal static void RecalculateFigureNumbers(XDocument document)
    {
        var next = 1;
        foreach (var element in document.Descendants("SECTION").SelectMany(s => s.Descendants()))
        {
            if (element.Ancestors().Any(a => a.Name.LocalName is "HEADER" or "FOOTER" or "MASTERPAGE")) continue;
            if (element.Name.LocalName == "NEWNUM" && (string?)element.Attribute("NumberType") == "Figure")
                next = (int)element.Attribute("Number")!;
            else if (element.Name.LocalName == "AUTONUM" && (string?)element.Attribute("NumberType") == "Figure")
                element.SetAttributeValue("Number", next++);
        }
    }

    internal static XElement ImportParagraph(XElement paragraph, XDocument source, XDocument destination)
    {
        string Import(string table, string id)
        {
            var original = source.Descendants(table).Single(e => (string?)e.Attribute("Id") == id);
            XElement Key(XElement e, XDocument doc)
            {
                var key = TemplateFormatting.Copy(e, doc);
                key.Attribute("Id")?.Remove();
                return key;
            }
            var key = Key(original, source);
            var found = destination.Descendants(table).FirstOrDefault(e => XNode.DeepEquals(key, Key(e, destination)));
            if (found is not null) return (string)found.Attribute("Id")!;
            var copy = new XElement(original);
            Resolve(copy);
            var list = destination.Descendants(table).FirstOrDefault()?.Parent
                ?? throw new InvalidDataException($"Missing template formatting table {table}.");
            var next = list.Elements(table).Max(e => (int)e.Attribute("Id")!) + 1;
            copy.SetAttributeValue("Id", next);
            list.Add(copy);
            list.SetAttributeValue("Count", list.Elements().Count());
            return next.ToString(System.Globalization.CultureInfo.InvariantCulture);
        }
        void Resolve(XElement root)
        {
            foreach (var element in root.DescendantsAndSelf())
            foreach (var attribute in element.Attributes().ToArray())
            {
                var table = attribute.Name.LocalName switch
                {
                    "ParaShape" => "PARASHAPE", "CharShape" => "CHARSHAPE", "TabDef" => "TABDEF",
                    "BorderFill" or "BorferFill" or "BorderFillId" when attribute.Value != "0" => "BORDERFILL",
                    "Heading" when element.Name.LocalName == "PARASHAPE" => (string?)element.Attribute("HeadingType") == "Bullet" ? "BULLET" : "NUMBERING",
                    _ => null,
                };
                if (table is not null) attribute.Value = Import(table, attribute.Value);
            }
        }
        var copy = new XElement(paragraph);
        Resolve(copy);
        var used = destination.Descendants().Attributes().Where(a => a.Name.LocalName is "InstId" or "InstID")
            .Select(a => a.Value).ToHashSet(StringComparer.Ordinal);
        foreach (var attribute in copy.DescendantsAndSelf().Attributes().Where(a => a.Name.LocalName is "InstId" or "InstID"))
        {
            string id;
            do { id = BitConverter.ToUInt32(Guid.NewGuid().ToByteArray(), 0).ToString(System.Globalization.CultureInfo.InvariantCulture); } while (!used.Add(id));
            attribute.Value = id;
        }
        return copy;
    }
}
