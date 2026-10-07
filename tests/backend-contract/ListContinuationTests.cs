using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class ListContinuationTests
{
    internal static void Run()
    {
        var source = new TemplateSource(XElement.Parse("<P><TEXT CharShape='0'><CHAR>source</CHAR></TEXT></P>"), "source", "", "");
        var profile = InvestigationTemplateProfile.FromTaggedTemplate(typeof(IrPreviewPlan).Assembly.Location,
            [], "reset", 100, 6, 0, new ProfileCaptionSelector("caption", "[", "]caption", "caption"),
            source, source, new TemplateListPrototype("bullet", 0, new XElement("BULLET")),
            new TemplateListPrototype("ordered", 0, new XElement("NUMBERING")));
        using var input = JsonDocument.Parse("""
            {"type":"list","kind":"ordered","start":9,"tight":false,"items":[{"blocks":[
            {"type":"paragraph","inlines":[{"type":"text","value":"first"}]},
            {"type":"paragraph","inlines":[{"type":"emph","inlines":[{"type":"text","value":"next"}]}]},
            {"type":"list","kind":"bullet","tight":false,"items":[{"blocks":[
            {"type":"paragraph","inlines":[{"type":"text","value":"child"}]},
            {"type":"paragraph","inlines":[{"type":"text","value":"childnext"}]}]}]},
            {"type":"paragraph","inlines":[{"type":"text","value":"parentnext"}]}]},
            {"blocks":[{"type":"paragraph","inlines":[{"type":"text","value":"ten"}]}]}]}
            """);
        var builder = new PlanBuilder("fixture.ir.json", Directory.GetCurrentDirectory(), profile);
        builder.AddBlock(input.RootElement, "/blocks/0");
        var operations = builder.Build(1).Operations;
        Check(operations[1].ListMarker is null && operations[1].ListContinuation == operations[0].ListMarker,
            "A second paragraph lost its item's identity or gained a new marker.");
        Check(operations[1].FormattedLines![0][0].Emphasis, "Continuation emphasis was lost.");
        Check(operations[3].ListContinuation == operations[2].ListMarker &&
            operations[3].ListContinuation!.Depth == 1, "Nested continuation lost its own depth.");
        Check(operations[4].ListContinuation == operations[0].ListMarker,
            "A continuation after a nested list followed the child instead of its parent.");
        Check(operations[5].ListMarker!.Number == 10 && operations[5].ListContinuation is null,
            "Continuation paragraphs changed the next item's number.");
        var head = XElement.Parse("<PARAHEAD TextOffset='50' TextOffsetType='percent' WidthAdjust='0'/>");
        Check(NativeListContinuations.CalculateMargin(0, 0, 1000, 1000, head) == 3000,
            "Marker widths were not converted to paragraph margin units.");
        head.SetAttributeValue("TextOffsetType", "hwpunit"); head.SetAttributeValue("TextOffset", "200");
        head.SetAttributeValue("WidthAdjust", "-100");
        Check(NativeListContinuations.CalculateMargin(7200, -200, 750, 1000, head) == 8700,
            "Nested margins, first-line indentation or absolute marker offsets were lost.");
        Console.WriteLine("List continuations retain parent/nested item identity, numbering, emphasis and native margin units.");
    }
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
}
