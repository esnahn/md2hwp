using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// A native list definition owned by one tagged template paragraph. Body styling
// and depth indentation remain independent of this numbering/bullet prototype.
internal sealed class TemplateListPrototype(string kind, int root, XElement definition)
{
    private object? nativeShape;
    public string Kind { get; } = kind;
    public XElement Definition { get; } = new(definition);

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

    internal void Verify(XElement actual, PreviewListMarker marker)
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
        if (!XNode.DeepEquals(expected, saved))
            throw new InvalidOperationException($"Native list.{Kind} definition differs from its template at depth {marker.Depth}: " +
                TemplateRangeStructure.DescribeDifference([expected], [saved]));
    }
}
