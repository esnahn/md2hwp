using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Lower the native caption sample to the existing text renderer, then attach its
// verified output to each generated picture. The intermediate form is never published.
internal sealed class NativeFigureCaption(XElement settings)
{
    internal static (NativeFigureCaption Layout, XDocument Document) Lower(XDocument source)
    {
        var document = new XDocument(source);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document).ToList();
        int Find(string token)
        {
            var matches = roots.Select((p, i) => (p, i)).Where(x => TaggedTemplateBinding.DirectText(x.p) == TaggedTemplateBinding.Tag(token)).ToArray();
            if (matches.Length != 1) throw new InvalidDataException($"Expected one {token} declaration. Regenerate with init-template.");
            var declaration = matches[0].p;
            if (declaration.Elements().Any(e => e.Name.LocalName != "TEXT") ||
                declaration.Elements().SelectMany(e => e.Elements()).Any(e => e.Name.LocalName != "CHAR" || e.HasElements))
                throw new InvalidDataException($"{token} must be a plain, single-line paragraph.");
            return matches[0].i;
        }
        var begin = Find("begin:figure.caption");
        var end = Find("end:figure.caption");
        if (end != begin + 2) throw new InvalidDataException("figure.caption must contain exactly one sample picture paragraph.");
        var root = roots[begin + 1];
        var pictures = root.Descendants("PICTURE").ToArray();
        if (pictures.Length != 1 || root.Elements().Any(e => e.Name.LocalName != "TEXT") ||
            root.Elements().SelectMany(e => e.Elements()).Any(e => e.Name.LocalName != "PICTURE" &&
                !(e.Name.LocalName == "CHAR" && !e.HasElements && e.Value.Length == 0)) ||
            root.Descendants("TABLE").Any() || TaggedTemplateBinding.DirectText(root).Length != 0)
            throw new InvalidDataException("figure.caption requires one sample picture without other content.");
        var shape = pictures[0].Element("SHAPEOBJECT") ?? throw new InvalidDataException("Sample picture has no shape settings.");
        var caption = shape.Element("CAPTION") ?? throw new InvalidDataException("Sample picture requires a native Hancom caption.");
        var paragraphs = caption.Element("PARALIST")?.Elements("P").ToArray() ?? [];
        if (paragraphs.Length != 2 || !TaggedTemplateBinding.DirectText(paragraphs[0]).Contains(TaggedTemplateBinding.Tag("slot:figure.caption"), StringComparison.Ordinal) ||
            !TaggedTemplateBinding.DirectText(paragraphs[1]).Contains(TaggedTemplateBinding.Tag("slot:figure.source"), StringComparison.Ordinal))
            throw new InvalidDataException("Native caption requires caption and source slot paragraphs, in that order.");
        XElement Declaration(XElement prototype, string token)
        {
            var p = new XElement(prototype.Name, prototype.Attributes());
            p.Add(new XElement("TEXT", new XAttribute("CharShape", (string?)prototype.Element("TEXT")?.Attribute("CharShape") ?? "0"),
                new XElement("CHAR", TaggedTemplateBinding.Tag(token))));
            return p;
        }
        // Removing the sample picture decreases the calculated Figure counter by one.
        foreach (var number in paragraphs[0].Descendants("AUTONUM"))
            number.SetAttributeValue("Number", Math.Max(0, (int?)number.Attribute("Number") - 1 ?? 0));
        roots[begin].ReplaceWith(Declaration(roots[begin], "begin:figure"));
        root.ReplaceWith(Declaration(root, "slot:figure.image"), new XElement(paragraphs[0]), new XElement(paragraphs[1]));
        roots[end].ReplaceWith(Declaration(roots[end], "end:figure"));
        return (new(new XElement(caption)), document);
    }

    internal XDocument Attach(XDocument rendered, IrPreviewPlan plan, int start)
    {
        var result = new XDocument(rendered);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(result);
        var index = start;
        int? lastChapter = null;
        foreach (var operation in plan.Operations)
        {
            if (operation.Kind != "figure") { index++; continue; }
            var root = roots[index];
            var shape = root.Descendants("PICTURE").Single().Element("SHAPEOBJECT")!;
            if (shape.Element("CAPTION") is not null) throw new InvalidOperationException("Generated picture already has a caption.");
            var caption = new XElement(settings);
            var paragraphs = new List<XElement> { new(roots[index + 1]) };
            var count = operation.Lines[2].Length > 0 ? 2 : 1;
            if (count == 2) paragraphs.Add(new(roots[index + 2]));
            caption.Element("PARALIST")!.ReplaceNodes(TemplateHeadingNumbers.Fill(paragraphs, operation.Heading1Number));
            if (operation.Heading1Number is not null && operation.Heading1Number != lastChapter)
            {
                root.Elements("TEXT").First().AddFirst(new XElement("NEWNUM",new XAttribute("Number","1"),new XAttribute("NumberType","Figure")));
                lastChapter = operation.Heading1Number;
            }
            // LastWidth is Hancom's calculated layout cache, not a user option.
            caption.SetAttributeValue("LastWidth", (string?)caption.Attribute("Side") is "Left" or "Right"
                ? (string?)caption.Attribute("Width") : (string?)shape.Element("SIZE")?.Attribute("Width"));
            var comment = shape.Element("SHAPECOMMENT");
            if (comment is not null) comment.AddBeforeSelf(caption); else shape.Add(caption);
            for (var i = 1; i <= count; i++) roots[index + i].Remove();
            index += count + 1;
        }
        return result;
    }
}
