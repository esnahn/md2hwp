using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Owns only the generated flat document. Each ordered-list segment gets an
// independent NUMBERING identity, including otherwise identical restarts.
internal sealed class DirectXmlLists
{
    private sealed record Item(PreviewListMarker Marker, XElement Paragraph, XElement Head, XElement CharacterShape,
        int Left, int Indent, int Height);
    private readonly XDocument document;
    private readonly ProfileListLayout layout;
    private readonly HashSet<(int List, int Number)> required;
    private readonly Dictionary<(int List, int Number), Item> items = [];
    private readonly Dictionary<(int List, int Number), int> margins = [];
    private readonly List<(XElement Paragraph, PreviewListMarker Marker)> continuationParagraphs = [];
    private readonly Dictionary<string, int> paragraphDefinitions = new(StringComparer.Ordinal);
    private int? activeList;
    private int activeDefinition;
    private int? bulletDefinition;

    internal DirectXmlLists(XDocument document, IrPreviewPlan plan, ProfileListLayout layout)
    {
        this.document = document;
        this.layout = layout;
        required = plan.Operations.Where(operation => operation.ListContinuation is not null)
            .Select(operation => (operation.ListContinuation!.ListId, operation.ListContinuation.Number)).ToHashSet();
        Continuations = new NativeListContinuations(document, plan);
        foreach (var definition in document.Descendants("PARASHAPE"))
            paragraphDefinitions.TryAdd(Key(definition), RequiredId(definition));
    }

    internal NativeListContinuations Continuations { get; }

    internal bool NeedsNativeMeasurement => items.Any(item => !margins.ContainsKey(item.Key));

    internal void BreakSequence() => activeList = null;

    internal static void Clear(XElement paragraph, XDocument document)
    {
        var original = document.Descendants("PARASHAPE").Single(value =>
            (string?)value.Attribute("Id") == (string?)paragraph.Attribute("ParaShape"));
        var shape = new XElement(original);
        shape.SetAttributeValue("HeadingType", "None");
        shape.Attribute("Heading")?.Remove();
        shape.SetAttributeValue("Level", 0);
        var key = Key(shape);
        var existing = original.Parent!.Elements("PARASHAPE").FirstOrDefault(value => Key(value) == key);
        var id = existing is null ? AppendDefinition(document, "PARASHAPE", shape) : RequiredId(existing);
        paragraph.SetAttributeValue("ParaShape", id);
    }

    internal void Apply(XElement paragraph, PreviewListMarker marker, int baseLeftMargin)
    {
        if (marker.Kind is not ("bullet" or "ordered") || marker.Depth < 0 || marker.Depth > layout.MaxDepth)
            throw new InvalidOperationException("Unsupported direct XML native list kind or depth.");
        var shape = Shape(paragraph);
        if (activeList != marker.ListId)
        {
            var definition = new XElement(layout.Prototype(marker.Kind).Definition);
            NormalizeMarkerBackground(definition);
            if (marker.Kind == "ordered")
            {
                definition.SetAttributeValue("Start", marker.Number);
                definition.Elements("PARAHEAD").Single(head => (int?)head.Attribute("Level") == marker.Depth + 1)
                    .SetAttributeValue("Start", marker.Number);
                activeDefinition = AppendDefinition("NUMBERING", definition);
            }
            else
            {
                bulletDefinition ??= AppendDefinition("BULLET", definition);
                activeDefinition = bulletDefinition.Value;
            }
            activeList = marker.ListId;
        }
        shape.SetAttributeValue("HeadingType", marker.Kind == "bullet" ? "Bullet" : "Number");
        shape.SetAttributeValue("Heading", activeDefinition);
        shape.SetAttributeValue("Level", marker.Depth);
        var left = checked(baseLeftMargin + marker.Depth * layout.DepthIndentHwpUnits);
        Margin(shape).SetAttributeValue("Left", left);
        SetShape(paragraph, shape);

        var key = (marker.ListId, marker.Number);
        if (!required.Contains(key)) return;
        var definitionHead = layout.Prototype(marker.Kind).Definition;
        var head = new XElement(marker.Kind == "bullet" ? definitionHead.Element("PARAHEAD")! :
            definitionHead.Elements("PARAHEAD").Single(value => (int?)value.Attribute("Level") == marker.Depth + 1));
        var character = document.Descendants("CHARSHAPE").Single(value =>
            (string?)value.Attribute("Id") == (string?)head.Attribute("CharShape"));
        var height = (int?)character.Attribute("Height") ??
            throw new InvalidOperationException("Native list marker has no character height.");
        var item = new Item(marker, paragraph, head, new XElement(character), left,
            (int?)Margin(shape).Attribute("Indent") ?? 0, height);
        if (!items.TryAdd(key, item))
            throw new InvalidOperationException("A direct XML list item was generated more than once.");
        if ((string?)head.Attribute("UseInstWidth") == "false") RecordMargin(key, item, height);
    }

    internal void ApplyContinuation(XElement paragraph, PreviewListMarker marker)
    {
        BreakSequence();
        var key = (marker.ListId, marker.Number);
        if (!items.ContainsKey(key))
            throw new InvalidOperationException("Missing first paragraph for a direct XML list continuation.");
        continuationParagraphs.Add((paragraph, marker));
        SetContinuation(paragraph, margins.GetValueOrDefault(key, items[key].Left));
    }

    // Import the flat document before this call only when actual marker display
    // width is needed. Cursor movement reads the native marker; it does not alter
    // either the imported content or the independent expected XML document.
    internal void ResolveContinuations(object automation, XDocument flat)
    {
        dynamic hwp = automation;
        var roots = AuriMinimalBoxPrototype.RootParagraphs(flat);
        foreach (var (key, item) in items)
        {
            if (margins.ContainsKey(key)) continue;
            var matches = roots.Select((paragraph, index) => (paragraph, index))
                .Where(value => ReferenceEquals(value.paragraph, item.Paragraph)).ToArray();
            if (matches.Length != 1)
                throw new InvalidOperationException("Cannot locate a direct XML list item in the flat document.");
            if (!(bool)hwp.SetPos(0, matches[0].index, 0))
                throw new InvalidOperationException("Cannot measure a direct XML native list marker.");
            var display = (string)hwp.GetHeadingString();
            if (string.IsNullOrWhiteSpace(display))
                throw new InvalidOperationException("Cannot measure an empty native list marker.");
            RecordMargin(key, item, TableAutoWidths.MeasurePlainText(document, item.CharacterShape, display));
        }
        foreach (var (paragraph, marker) in continuationParagraphs)
            SetContinuation(paragraph, margins[(marker.ListId, marker.Number)]);
    }

    private void RecordMargin((int List, int Number) key, Item item, double width)
    {
        var margin = NativeListContinuations.CalculateMargin(item.Left, item.Indent, width, item.Height, item.Head);
        margins[key] = margin;
        Continuations.RecordMargin(item.Marker, margin);
    }

    private void SetContinuation(XElement paragraph, int left)
    {
        var shape = Shape(paragraph);
        shape.SetAttributeValue("HeadingType", "None");
        shape.Attribute("Heading")?.Remove();
        shape.SetAttributeValue("Level", 0);
        var margin = Margin(shape);
        margin.SetAttributeValue("Left", left);
        margin.SetAttributeValue("Indent", 0);
        SetShape(paragraph, shape);
    }

    private XElement Shape(XElement paragraph) => new(document.Descendants("PARASHAPE").Single(value =>
        (string?)value.Attribute("Id") == (string?)paragraph.Attribute("ParaShape")));

    private static XElement Margin(XElement shape) => shape.Element("PARAMARGIN") ??
        throw new InvalidOperationException("Native paragraph shape has no margin definition.");

    private void SetShape(XElement paragraph, XElement shape)
    {
        var key = Key(shape);
        if (!paragraphDefinitions.TryGetValue(key, out var id))
        {
            id = AppendDefinition("PARASHAPE", shape);
            paragraphDefinitions.Add(key, id);
        }
        paragraph.SetAttributeValue("ParaShape", id);
    }

    private int AppendDefinition(string name, XElement definition) => AppendDefinition(document, name, definition);

    private void NormalizeMarkerBackground(XElement definition)
    {
        // Native BulletShape/NumberingShape serializes the transparent default
        // background without WINDOWBRUSH. Match that existing COM representation
        // on private marker formats only; explicit fills remain untouched.
        foreach (var head in definition.Elements("PARAHEAD"))
        {
            var character = document.Descendants("CHARSHAPE").Single(value =>
                (string?)value.Attribute("Id") == (string?)head.Attribute("CharShape"));
            var borderId = character.Attribute("BorderFillId");
            if (borderId is null) continue;
            var border = document.Descendants("BORDERFILL").SingleOrDefault(value =>
                (string?)value.Attribute("Id") == borderId.Value);
            var brush = border?.Element("FILLBRUSH");
            var window = brush?.Element("WINDOWBRUSH");
            if (brush is null || brush.Attributes().Any() || brush.Elements().Count() != 1 ||
                window is null || window.HasElements || window.Attributes().Count() != 3 ||
                (string?)window.Attribute("Alpha") != "0" ||
                (string?)window.Attribute("FaceColor") != "4294967295" ||
                (string?)window.Attribute("HatchColor") != "4278190080") continue;
            var cleanBorder = new XElement(border!);
            cleanBorder.Element("FILLBRUSH")!.Remove();
            var cleanCharacter = new XElement(character);
            cleanCharacter.SetAttributeValue("BorderFillId", InternDefinition("BORDERFILL", cleanBorder));
            head.SetAttributeValue("CharShape", InternDefinition("CHARSHAPE", cleanCharacter));
        }
    }

    private int InternDefinition(string name, XElement definition)
    {
        var key = Key(definition);
        var existing = document.Descendants(name).FirstOrDefault(value => Key(value) == key);
        return existing is null ? AppendDefinition(name, definition) : RequiredId(existing);
    }

    private static int AppendDefinition(XDocument document, string name, XElement definition)
    {
        var parent = document.Descendants(name + "LIST").Single();
        var id = checked(parent.Elements(name).Select(RequiredId).DefaultIfEmpty(-1).Max() + 1);
        definition.SetAttributeValue("Id", id);
        foreach (var element in definition.DescendantsAndSelf())
            element.ReplaceAttributes(element.Attributes().OrderBy(attribute => attribute.Name.LocalName, StringComparer.Ordinal)
                .Select(attribute => new XAttribute(attribute)).ToArray());
        parent.Add(definition);
        parent.SetAttributeValue("Count", parent.Elements(name).Count());
        return id;
    }

    private static int RequiredId(XElement value) => (int?)value.Attribute("Id") ??
        throw new InvalidOperationException($"Native {value.Name} definition has no identity.");

    private static string Key(XElement value)
    {
        var copy = new XElement(value);
        copy.Attribute("Id")?.Remove();
        foreach (var element in copy.DescendantsAndSelf())
            element.ReplaceAttributes(element.Attributes().OrderBy(attribute => attribute.Name.ToString(), StringComparer.Ordinal)
                .Select(attribute => new XAttribute(attribute)).ToArray());
        return copy.ToString(SaveOptions.DisableFormatting);
    }
}
