using System.Globalization;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed class NativeListContinuations(XDocument document, IrPreviewPlan plan)
{
    private readonly HashSet<(int, int)> required = plan.Operations.Where(o => o.ListContinuation is not null)
        .Select(o => (o.ListContinuation!.ListId, o.ListContinuation.Number)).ToHashSet();
    private readonly Dictionary<(int, int), int> margins = [];

    internal int Margin(PreviewListMarker marker) => margins.TryGetValue((marker.ListId, marker.Number), out var value)
        ? value : throw new InvalidOperationException("Missing first paragraph for a list continuation.");

    internal void Record(dynamic hwp, PreviewListMarker marker, ProfileListLayout lists)
    {
        if (!required.Contains((marker.ListId, marker.Number))) return;
        var definition = lists.Prototype(marker.Kind).Definition;
        var head = marker.Kind == "bullet" ? definition.Element("PARAHEAD")! : definition.Elements("PARAHEAD")
            .Single(h => (int?)h.Attribute("Level") == marker.Depth + 1);
        var shape = document.Descendants("CHARSHAPE").Single(c =>
            (string?)c.Attribute("Id") == (string?)head.Attribute("CharShape"));
        var height = (int)shape.Attribute("Height")!;
        var useWidth = (string?)head.Attribute("UseInstWidth") != "false";
        var display = useWidth ? (string)hwp.GetHeadingString() : "";
        if (useWidth && string.IsNullOrWhiteSpace(display))
            throw new InvalidOperationException("Cannot measure an empty native list marker.");
        var width = useWidth ? TableAutoWidths.MeasurePlainText(document, shape, display) : height;
        dynamic action = hwp.CreateAction("ParagraphShape");
        dynamic parameters = action.CreateSet();
        action.GetDefault(parameters);
        var left = Convert.ToInt32(parameters.Item("LeftMargin"), CultureInfo.InvariantCulture);
        var indent = Convert.ToInt32(parameters.Item("Indentation"), CultureInfo.InvariantCulture);
        margins[(marker.ListId, marker.Number)] = CalculateMargin(left, indent, width, height, head);
    }

    internal static int CalculateMargin(int left, int indent, double width, int height, XElement head)
    {
        var offset = (double?)head.Attribute("TextOffset") ?? 50;
        if ((string?)head.Attribute("TextOffsetType") != "hwpunit") offset = height * offset / 100;
        // Paragraph margins use URC (1/14400 inch); marker metrics use HWPUNIT.
        return checked(left + indent + (int)Math.Round(2 * (width + offset + ((int?)head.Attribute("WidthAdjust") ?? 0))));
    }

    internal void Apply(dynamic hwp, PreviewListMarker marker)
    {
        dynamic action = hwp.CreateAction("ParagraphShape");
        dynamic parameters = action.CreateSet();
        action.GetDefault(parameters);
        parameters.SetItem("HeadingType", 0);
        parameters.SetItem("Level", 0);
        parameters.SetItem("LeftMargin", Margin(marker));
        parameters.SetItem("Indentation", 0);
        if (!(bool)action.Execute(parameters))
            throw new InvalidOperationException("Could not align a list continuation paragraph.");
    }
}
