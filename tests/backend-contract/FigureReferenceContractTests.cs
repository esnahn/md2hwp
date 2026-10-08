using System.Text.Json;
using Md2Hwp.HancomIrPreview;

internal static class FigureReferenceContractTests
{
    internal static void Run()
    {
        static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
        static void Reject(Action action) { try { action(); } catch (InvalidDataException) { return; } throw new Exception("Expected reference-contract rejection."); }
        foreach (var id in new[] { "plain", "한글-ID", "fig:heading", "표현%id", "e\u0301", "😀" }) FigureReferenceContract.RequireId(id, "/id");
        foreach (var id in new[] { "", "two words", "a\tb", "#x", "a\u00a0b", "a\u007fb", "\ud800" }) Reject(() => FigureReferenceContract.RequireId(id, "/id"));
        using (var id = JsonDocument.Parse("{\"id\":\"한글\"}")) Check(FigureReferenceContract.ReadOptionalId(id.RootElement, "") == "한글", "Opaque Unicode ID changed.");
        using (var id = JsonDocument.Parse("{}")) Check(FigureReferenceContract.ReadOptionalId(id.RootElement, "") is null, "Missing optional ID acquired a value.");
        using (var id = JsonDocument.Parse("{\"id\":null}")) Reject(() => FigureReferenceContract.ReadOptionalId(id.RootElement, ""));
        static PreviewInlineContent Read(string json, bool allowed = true)
        {
            using var input = JsonDocument.Parse(json);
            return InlineText.Read(input.RootElement, "/inlines", allowCrossReferences: allowed);
        }
        var input = "[{\"type\":\"strong\",\"inlines\":[{\"type\":\"cross_reference\",\"kind\":\"heading_number\",\"target\":\"fig:heading\"}]},{\"type\":\"space\"},{\"type\":\"cross_reference\",\"kind\":\"figure_number\",\"target\":\"plain\"},{\"type\":\"text\",\"value\":\"end\"}]";
        var runs = Read(input).Lines.Single().Runs;
        Check(runs.Count == 4 && runs[0].Strong && runs[0].CrossReference?.Kind == "heading_number" && runs[2].CrossReference?.Target == "plain", "Reference metadata or formatting was coalesced into ordinary text.");
        Check(runs[0].Text.StartsWith("MD2HWP_CROSS_REFERENCE_", StringComparison.Ordinal) && runs[0].Text != runs[2].Text, "Reference placeholders are not distinct.");
        var operations = new[] {
            new PreviewOperation("text", "reader", [], FormattedLines: new[] { runs }),
            new PreviewOperation("text", "heading", [], ParagraphStyle: "heading2", HeadingId: "fig:heading"),
            new PreviewOperation("figure", "figure", [], FigureId: "plain") };
        FigureReferenceContract.Validate(operations);
        Reject(() => FigureReferenceContract.Validate(operations[..2]));
        Reject(() => FigureReferenceContract.Validate(operations.Concat(new[] { new PreviewOperation("figure", "duplicate", [], FigureId: "fig:heading") }).ToArray()));
        Reject(() => FigureReferenceContract.Validate(new[] { operations[0], operations[1] with { HeadingId = "plain" }, operations[2] with { FigureId = "fig:heading" } }));
        var tableRuns = Read("[{\"type\":\"cross_reference\",\"kind\":\"table_number\",\"target\":\"표\"}]").Lines.Single().Runs;
        var tableTarget = new PreviewOperation("table", "table", [], TableId: "표");
        var tableReader = new PreviewOperation("text", "reader", [], FormattedLines: [tableRuns]);
        FigureReferenceContract.Validate([tableReader, tableTarget]);
        Reject(() => FigureReferenceContract.Validate([tableReader]));
        Reject(() => FigureReferenceContract.Validate([tableReader, tableTarget, operations[2] with { FigureId = "표" }]));
        var note = Read("[{\"type\":\"footnote\",\"blocks\":[{\"type\":\"paragraph\",\"inlines\":[{\"type\":\"cross_reference\",\"kind\":\"heading_number\",\"target\":\"fig:heading\"}]}]}]");
        FigureReferenceContract.Validate(new[] { operations[1], new PreviewOperation("text", "note", [], FormattedLines: note.Lines.Select(l => l.Runs).ToArray()) });
        Reject(() => FigureReferenceContract.Validate(new[] { new PreviewOperation("text", "note", [], FormattedLines: note.Lines.Select(l => l.Runs).ToArray()) }));
        var first = new PreviewSourceParagraph("출처", Read("[{\"type\":\"text\",\"value\":\"plain\"}]"));
        var second = new PreviewSourceParagraph("주", Read(input));
        var sourced = new PreviewOperation("code", "sources", [], Sources: [first, second]);
        FigureReferenceContract.Validate([sourced, operations[1], operations[2]]);
        Reject(() => FigureReferenceContract.Validate([sourced, operations[2]]));
        Check(FigureReferenceContract.ReadRuns([sourced]).Count(r => r.CrossReference is not null) == 2, "References in later object notes disappeared.");
        var altOnly = operations[2] with { FormattedLines = [runs, Array.Empty<PreviewTextRun>()] };
        Check(!FigureReferenceContract.ReadRuns([altOnly]).Any(), "Figure alt was mistaken for rendered caption text.");
        Reject(() => FigureReferenceContract.Validate([altOnly]));
        Reject(() => Read(input, false));
        Reject(() => Read("[{\"type\":\"link\",\"target\":\"https://example.org\",\"title\":null,\"inlines\":[{\"type\":\"cross_reference\",\"kind\":\"figure_number\",\"target\":\"plain\"}]}]", false));
        foreach (var json in new[] {

            "[{\"type\":\"cross_reference\",\"kind\":\"figure_page\",\"target\":\"plain\"}]",
            "[{\"type\":\"cross_reference\",\"kind\":\"figure_number\",\"target\":\"\"}]",
            "[{\"type\":\"cross_reference\",\"kind\":\"figure_number\",\"target\":\"plain\",\"label\":\"bad\"}]" }) Reject(() => Read(json));
        Console.WriteLine("Backend opaque IDs, forward/native reference kinds and note context contracts passed.");
    }
}
