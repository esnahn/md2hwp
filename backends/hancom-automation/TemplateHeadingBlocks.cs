using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Optional, template-only presentation of an existing IR heading.
internal sealed class TemplateHeadingBlocks(XDocument source, Dictionary<string, XElement[]> samples)
{
    internal static (TemplateHeadingBlocks Layout, XDocument Document) Lower(XDocument source)
    {
        var document = new XDocument(source);
        var samples = new Dictionary<string, XElement[]>(StringComparer.Ordinal);
        for (var level = 1; level <= 6; level++)
        {
            var role = $"heading.{level}";
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
            foreach (var paragraph in roots.Skip(first).Take(last - first + 1))
                if (paragraph.Elements().Any(e => e.Name.LocalName != "TEXT") ||
                    paragraph.Elements().SelectMany(e => e.Elements()).Any(e => e.Name.LocalName != "CHAR" || e.HasElements))
                    throw new InvalidDataException($"{role} block supports plain text paragraphs only; objects, controls and manual line breaks are not supported.");
            var slot = TaggedTemplateBinding.Tag("slot:" + role);
            var slots = range.Where(p => TaggedTemplateBinding.DirectText(p) == slot).ToArray();
            if (slots.Length != 1 || range.Any(p => p != slots[0] && TaggedTemplateBinding.DirectText(p).Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal)))
                throw new InvalidDataException($"{role} block requires one standalone slot:{role} paragraph and no nested declarations.");
            samples.Add(role, range.Select(p => new XElement(p)).ToArray());
            var declaration = new XElement(slots[0]);
            declaration.ReplaceNodes(new XElement("TEXT", new XAttribute("CharShape", (string?)slots[0].Element("TEXT")?.Attribute("CharShape") ?? "0"),
                new XElement("CHAR", TaggedTemplateBinding.Tag(role))));
            begin[0].ReplaceWith(declaration);
            foreach (var paragraph in range) paragraph.Remove();
            end[0].Remove();
        }
        return (new(new XDocument(source), samples), document);
    }

    // Native figure captions are already attached: each operation now owns one root.
    internal XDocument Attach(XDocument rendered, IrPreviewPlan plan, int start)
    {
        var result = new XDocument(rendered);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(result).ToArray();
        for (var index = 0; index < plan.Operations.Count; index++)
        {
            var operation = plan.Operations[index];
            if (operation.Kind != "text" || operation.ParagraphStyle is null || !samples.TryGetValue(operation.ParagraphStyle, out var sample)) continue;
            var generated = roots[start + index];
            var slot = TaggedTemplateBinding.Tag("slot:" + operation.ParagraphStyle);
            var block = sample.Select(p => ImportParagraph(p, source, result)).ToArray();
            var target = block.Single(p => TaggedTemplateBinding.DirectText(p) == slot);
            target.ReplaceNodes(generated.Nodes().Select(n => n is XElement e ? new XElement(e) : throw new InvalidDataException("Unexpected heading node.")));
            generated.ReplaceWith(block);
        }
        return result;
    }

    private static XElement ImportParagraph(XElement paragraph, XDocument source, XDocument destination)
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
                    "BorderFill" or "BorferFill" when attribute.Value != "0" => "BORDERFILL",
                    "Heading" when element.Name.LocalName == "PARASHAPE" => (string?)element.Attribute("HeadingType") == "Bullet" ? "BULLET" : "NUMBERING",
                    _ => null,
                };
                if (table is not null) attribute.Value = Import(table, attribute.Value);
            }
        }
        var copy = new XElement(paragraph);
        Resolve(copy);
        return copy;
    }
}
