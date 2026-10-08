using Md2Hwp.HancomIrPreview;

internal static class HeadingReferenceNumbersTests
{
    internal static void Run()
    {
        static PreviewOperation Heading(int level, string? id, int chapter) => new("text", "heading", ["title"], ParagraphStyle: $"heading{level}", Heading1Number: chapter, HeadingId: id);
        var operations = new[] { Heading(1, "chapter3", 3), Heading(2, null, 3), Heading(2, "section3", 3), Heading(3, "detail", 3),
            Heading(2, "next-section", 3), Heading(3, "reset-detail", 3), Heading(4, "four", 3), Heading(5, "five", 3), Heading(6, "six", 3),
            Heading(1, "chapter4", 4), Heading(2, "section4", 4) };
        var targets = HeadingReferenceNumbers.Count(operations);
        Check(targets["section3"].Numbers.Take(2).SequenceEqual(new[] { 3, 2 }) && targets["detail"].Numbers.Take(3).SequenceEqual(new[] { 3, 2, 1 }), "Unidentified headings were skipped or chapter numbering was wrong.");
        Check(targets["reset-detail"].Numbers.Take(3).SequenceEqual(new[] { 3, 3, 1 }) && targets["section4"].Numbers.Take(2).SequenceEqual(new[] { 4, 1 }), "Counters did not reset after parent headings.");
        Check(targets["six"].Numbers.SequenceEqual(new[] { 3, 3, 1, 1, 1, 1 }), "Six-level hierarchy was not counted.");
        Check(HeadingReferenceNumbers.Count([new PreviewOperation("text", "heading", [], ParagraphStyle: "heading3", HeadingId: "skip")])["skip"].Numbers.Take(3).SequenceEqual(new[] { 0, 0, 1 }), "Skipped parent levels did not retain zero counters.");
        const string marker = "MD2HWP_CROSS_REFERENCE_33333333333333333333333333333333";
        var run = new PreviewTextRun(marker, true, true, CrossReference: new("heading_number", "section4"));
        var content = new PreviewInlineContent([new(marker, [run])]);
        var note = new PreviewTextRun("NOTE_MARKER", false, false, Footnote: new([content]));
        var reader = new PreviewOperation("text", "reader", [marker + " NOTE_MARKER"], FormattedLines: [[run, new(" ", false, false), note]],
            Sources: [new("출처", content)]);
        var table = new PreviewOperation("table", "table", ["TABLE_MARKER"], Table: new(["default"], [content], [[content]], content, content));
        var code = new PreviewOperation("code", "verbatim", [marker], Sources: [new("참고", content)]);
        var plan = new IrPreviewPlan("fixture", "fixture", new(14, 12, 1, 0, 0, 1), new[] { reader, table, code }.Concat(operations).ToArray(), []);
        var template = TemplateCrossReferences.Lower(TemplateCrossReferenceTests.Fixture()).Layout;
        var bound = HeadingReferenceNumbers.Bind(plan, template);
        Check(bound.Operations[0].Lines.Single() == "4.1절 NOTE_MARKER" && bound.Operations[0].FormattedLines![0][0] is { Text: "4.1절", Strong: true, Emphasis: true, CrossReference: null }, "Forward/cross-chapter reference or emphasis was lost.");
        Check(bound.Operations[0].FormattedLines![0][2].Footnote!.Paragraphs[0].Lines[0].Text == "4.1절" &&
            bound.Operations[1].Table!.Header[0].Lines[0].Text == "4.1절" && bound.Operations[1].Table!.Caption!.Lines[0].Text == "4.1절" &&
            bound.Operations[2].Sources![0].Content.Lines[0].Text == "4.1절", "Notes, table cells/captions or object notes retained symbolic heading references.");
        Check(bound.Operations[2].Lines.Single() == marker && plan.Operations[0].FormattedLines![0][0].CrossReference is not null,
            "Binding changed verbatim body literals or its input plan.");
        Check(!FigureReferenceContract.ReadRuns(bound.Operations).Any(run => run.CrossReference?.Kind == "heading_number"), "Native heading reference remains after binding.");
        Console.WriteLine("Manuscript heading counters, resets, forward references, literal preservation and all inline contexts passed.");
    }
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
}
