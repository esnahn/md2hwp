namespace Md2Hwp.HancomIrPreview;

// Heading order in the manuscript owns these fixed reference numbers, not native outlines.
internal static class HeadingReferenceNumbers
{
    internal sealed record Target(int Level, int[] Numbers);
    internal static Dictionary<string, Target> Count(IReadOnlyList<PreviewOperation> operations)
    {
        var result = new Dictionary<string, Target>(StringComparer.Ordinal);
        var counters = new int[6];
        foreach (var operation in operations)
        {
            var role = operation.ParagraphStyle;
            if (operation.Kind != "text" || role is null || role.Length != 8 || !role.StartsWith("heading", StringComparison.Ordinal) || role[7] is < '1' or > '6') continue;
            var index = role[7] - '1';
            if (index == 0) counters[0] = operation.Heading1Number ?? checked(counters[0] + 1);
            else
            {
                if (operation.Heading1Number is { } chapter) counters[0] = chapter;
                counters[index] = checked(counters[index] + 1);
            }
            Array.Clear(counters, index + 1, counters.Length - index - 1);
            if (operation.HeadingId is { } id && !result.TryAdd(id, new(index + 1, (int[])counters.Clone())))
                throw new InvalidDataException("Duplicate heading reference target.");
        }
        return result;
    }

    internal static IrPreviewPlan Bind(IrPreviewPlan plan, TemplateCrossReferences template)
    {
        FigureReferenceContract.Validate(plan.Operations);
        var targets = Count(plan.Operations);
        var substitutions = new Dictionary<string, string>(StringComparer.Ordinal);
        PreviewTextRun Run(PreviewTextRun run)
        {
            if (run.CrossReference is { Kind: "heading_number" } reference)
            {
                if (!targets.TryGetValue(reference.Target, out var target)) throw new InvalidDataException("Missing heading reference target.");
                var text = template.HeadingText(target.Level, target.Numbers);
                if (substitutions.TryGetValue(run.Text, out var old) && old != text) throw new InvalidDataException("Ambiguous heading reference marker.");
                substitutions[run.Text] = text;
                return run with { Text = text, CrossReference = null };
            }
            return run.Footnote is null ? run : run with { Footnote = run.Footnote with { Paragraphs = run.Footnote.Paragraphs.Select(Content).ToArray() } };
        }
        PreviewInlineContent Content(PreviewInlineContent content) => content with { Lines = content.Lines.Select(line =>
        {
            var runs = line.Runs.Select(Run).ToArray();
            return line with { Runs = runs, Text = string.Concat(runs.Select(run => run.Text)) };
        }).ToArray() };
        var operations = plan.Operations.Select(operation => operation with
        {
            FormattedLines = operation.FormattedLines?.Select(line => (IReadOnlyList<PreviewTextRun>)line.Select(Run).ToArray()).ToArray(),
            SourceRuns = operation.SourceRuns?.Select(Run).ToArray(),
            Sources = operation.Sources?.Select(source => source with { Content = Content(source.Content) }).ToArray(),
            Table = operation.Table is not { } table ? null : table with
            {
                Header = table.Header.Select(Content).ToArray(),
                Rows = table.Rows.Select(row => (IReadOnlyList<PreviewInlineContent>)row.Select(Content).ToArray()).ToArray(),
                Caption = table.Caption is null ? null : Content(table.Caption),
                Source = table.Source is null ? null : Content(table.Source)
            }
        }).ToArray();
        string Replace(string text)
        {
            foreach (var pair in substitutions) text = text.Replace(pair.Key, pair.Value, StringComparison.Ordinal);
            return text;
        }
        return plan with { Operations = operations.Select(operation => operation.Kind == "code" ? operation :
            operation with { Lines = operation.Lines.Select(Replace).ToArray() }).ToArray() };
    }
}
