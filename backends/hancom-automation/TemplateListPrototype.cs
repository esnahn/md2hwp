using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// A native list definition owned by one tagged template paragraph. Body styling
// and depth indentation remain independent of this numbering/bullet prototype.
internal sealed class TemplateListPrototype(string kind, int root, XElement definition)
{
    private object? nativeShape;
    public string Kind { get; } = kind;
    public XElement Definition { get; } = new(definition);

    // An omitted marker CharShape follows the paragraph's editing state in
    // Hancom. Resolve it from the sample, using private copies of the native
    // definitions so unrelated paragraphs retain their original behavior.
    internal static XDocument FreezeMarkerFormatting(XDocument source)
    {
        var document = new XDocument(source);
        foreach (var kind in new[] { "bullet", "ordered" })
        {
            var paragraph = document.Descendants("SECTION").Elements("P").Single(p =>
                TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("list." + kind));
            var shape = document.Descendants("PARASHAPE").Single(p =>
                (string?)p.Attribute("Id") == (string?)paragraph.Attribute("ParaShape"));
            var name = kind == "bullet" ? "BULLET" : "NUMBERING";
            var definition = document.Descendants(name).Single(d =>
                (string?)d.Attribute("Id") == (string?)shape.Attribute("Heading"));
            if (definition.Elements("PARAHEAD").All(h => h.Attribute("CharShape") is not null)) continue;
            var character = paragraph.Elements("TEXT").First(t => t.Elements("CHAR").Any()).Attribute("CharShape")?.Value
                ?? throw new InvalidDataException($"list.{kind} sample has no character formatting.");
            if (!document.Descendants("CHARSHAPE").Any(c => (string?)c.Attribute("Id") == character))
                throw new InvalidDataException($"list.{kind} sample references missing character formatting.");
            var fixedDefinition = new XElement(definition);
            var definitionId = document.Descendants(name).Max(d => (int)d.Attribute("Id")!) + 1;
            fixedDefinition.SetAttributeValue("Id", definitionId);
            foreach (var head in fixedDefinition.Elements("PARAHEAD").Where(h => h.Attribute("CharShape") is null))
            {
                head.SetAttributeValue("CharShape", character);
                head.ReplaceAttributes(head.Attributes().OrderBy(a => a.Name.LocalName, StringComparer.Ordinal)
                    .Select(a => new XAttribute(a)).ToArray());
            }
            definition.Parent!.Add(fixedDefinition);
            definition.Parent.SetAttributeValue("Count", definition.Parent.Elements(name).Count());
            var fixedShape = new XElement(shape);
            var shapeId = document.Descendants("PARASHAPE").Max(p => (int)p.Attribute("Id")!) + 1;
            fixedShape.SetAttributeValue("Id", shapeId);
            fixedShape.SetAttributeValue("Heading", definitionId);
            shape.Parent!.Add(fixedShape);
            shape.Parent.SetAttributeValue("Count", shape.Parent.Elements("PARASHAPE").Count());
            paragraph.SetAttributeValue("ParaShape", shapeId);
        }
        return document;
    }

    internal static TemplateListPrototype Read(XDocument document, XElement paragraph, int root, string kind, int maxDepth)
    {
        var shape = document.Descendants().SingleOrDefault(e => e.Name.LocalName == "PARASHAPE" &&
            (string?)e.Attribute("Id") == (string?)paragraph.Attribute("ParaShape"));
        var expected = kind == "bullet" ? "Bullet" : "Number";
        if ((string?)shape?.Attribute("HeadingType") != expected)
            throw new InvalidDataException($"list.{kind} must have a native {expected} marker. Create a new template with init-template or apply a native list to its sample paragraph.");
        var name = kind == "bullet" ? "BULLET" : "NUMBERING";
        var definition = document.Descendants().SingleOrDefault(e => e.Name.LocalName == name &&
            (string?)e.Attribute("Id") == (string?)shape!.Attribute("Heading"))
            ?? throw new InvalidDataException($"list.{kind} references a missing native definition.");
        if (kind == "ordered")
            for (var level = 1; level <= maxDepth + 1; level++)
            {
                var head = definition.Elements().SingleOrDefault(e => e.Name.LocalName == "PARAHEAD" && (int?)e.Attribute("Level") == level);
                if (head is null || string.IsNullOrEmpty(head.Value) || head.Attribute("NumFormat") is null)
                    throw new InvalidDataException($"list.ordered requires a number format for level {level}.");
            }
        return new(kind, root, definition);
    }

    internal void Bind(dynamic hwp)
    {
        if (!(bool)hwp.SetPos(0, root, 0)) throw new InvalidOperationException($"Cannot bind list.{Kind}.");
        dynamic action = hwp.CreateAction("ParagraphShape");
        dynamic parameters = action.CreateSet();
        action.GetDefault(parameters);
        dynamic subset = parameters.Item(Kind == "bullet" ? "Bullet" : "Numbering");
        nativeShape = subset.Clone();
    }

    internal void Apply(dynamic hwp, PreviewListMarker marker, NativeStyle body, ProfileListLayout layout)
    {
        if (nativeShape is null) throw new InvalidOperationException($"Unbound list.{Kind} prototype.");
        dynamic action = hwp.CreateAction("ParagraphShape");
        dynamic parameters = action.CreateSet();
        action.GetDefault(parameters);
        dynamic subset = parameters.CreateItemSet(Kind == "bullet" ? "Bullet" : "Numbering",
            Kind == "bullet" ? "BulletShape" : "NumberingShape");
        subset.Merge(nativeShape);
        parameters.SetItem("HeadingType", Kind == "bullet" ? 3 : 2);
        parameters.SetItem("Level", marker.Depth);
        parameters.SetItem("LeftMargin", body.BaseLeftMargin + marker.Depth * layout.DepthIndentHwpUnits);
        if (!(bool)action.Execute(parameters)) throw new InvalidOperationException($"Cannot apply list.{Kind} prototype.");
    }

    internal string StringFormat(int depth) => (string)((dynamic)nativeShape!).Item($"StrFormatLevel{depth}");

    internal ushort NumberFormat(int depth) => Convert.ToUInt16(((dynamic)nativeShape!).Item($"NumFormatLevel{depth}"));

    internal void Verify(XElement actual, PreviewListMarker marker, XDocument document)
    {
        var expected = new XElement(Definition);
        var saved = new XElement(actual);
        expected.Attribute("Id")?.Remove();
        saved.Attribute("Id")?.Remove();
        if (Kind == "ordered")
        {
            expected.SetAttributeValue("Start", marker.Number);
            expected.Elements().Single(e => e.Name.LocalName == "PARAHEAD" &&
                (int?)e.Attribute("Level") == marker.Depth + 1).SetAttributeValue("Start", marker.Number);
        }
        var comparison = new XDocument(document);
        // BulletShape serializes an unset character background without the
        // default WINDOWBRUSH that body CharShape exports. Preserve real fills.
        foreach (var brush in comparison.Descendants("BORDERFILL").Elements("FILLBRUSH").ToArray())
        {
            var window = brush.Element("WINDOWBRUSH");
            if (brush.Attributes().Any() || brush.Elements().Count() != 1 || window is null ||
                window.HasElements || window.Attributes().Count() != 3) continue;
            if ((string?)window.Attribute("Alpha") == "0" &&
                (string?)window.Attribute("FaceColor") == "4294967295" &&
                (string?)window.Attribute("HatchColor") == "4278190080") brush.Remove();
        }
        if (!TemplateRangeStructure.Equivalent([expected], [saved], comparison, comparison))
            throw new InvalidOperationException($"Native list.{Kind} definition differs from its template at depth {marker.Depth}: " +
                TemplateRangeStructure.DescribeDifference([expected], [saved], comparison, comparison));
    }
}
