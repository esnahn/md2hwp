using System.Text;
using System.Text.Json;

namespace Md2Hwp.HancomIrPreview;

// IDs are opaque decoded CommonMark identifiers. Target kinds come from the IR
// blocks themselves, never from an identifier prefix.
internal static class FigureReferenceContract
{
    internal static string? ReadOptionalId(JsonElement block, string path)
    {
        if (!block.TryGetProperty("id", out var value)) return null;
        var id = JsonContract.ReadString(value, path + "/id");
        RequireId(id, path + "/id");
        return id;
    }

    internal static void RequireId(string id, string path)
    {
        if (id.Length == 0 || id.Contains('#') || id.EnumerateRunes().Any(r => Rune.IsWhiteSpace(r) || Rune.IsControl(r)))
            throw JsonContract.Error(path, "identifier must be nonempty Unicode text without whitespace, controls or #");
        for (var i = 0; i < id.Length; i++)
        {
            if (!char.IsSurrogate(id[i])) continue;
            if (!char.IsHighSurrogate(id[i]) || i + 1 >= id.Length || !char.IsLowSurrogate(id[++i]))
                throw JsonContract.Error(path, "identifier must contain only Unicode scalar values");
        }
    }

    internal static void Validate(IReadOnlyList<PreviewOperation> operations)
    {
        var targets = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var operation in operations)
        {
            void Add(string? id, string kind)
            {
                if (id is null) return;
                RequireId(id, "/blocks/id");
                if (!targets.TryAdd(id, kind)) throw JsonContract.Error("/blocks/id", $"duplicate target identifier {id}");
            }
            Add(operation.FigureId, "figure_number");
            Add(operation.HeadingId, "heading_number");
        }
        foreach (var run in ReadRuns(operations))
        {
            if (run.CrossReference is not { } reference) continue;
            RequireId(reference.Target, "/inlines/target");
            if (reference.Kind is not ("figure_number" or "heading_number"))
                throw JsonContract.Error("/inlines/kind", "supported reference kinds are figure_number and heading_number");
            if (!targets.TryGetValue(reference.Target, out var actualKind))
                throw JsonContract.Error("/inlines/target", $"unknown reference target {reference.Target}");
            if (actualKind != reference.Kind)
                throw JsonContract.Error("/inlines/kind", $"reference {reference.Target} requires {actualKind}");
        }
    }

    internal static IEnumerable<PreviewTextRun> ReadRuns(IEnumerable<PreviewOperation> operations) =>
        operations.SelectMany(o => (o.FormattedLines ?? []).SelectMany(r => r).Concat(o.SourceRuns ?? [])
            .Concat(o.Table is null ? [] : o.Table.Header.Concat(o.Table.Rows.SelectMany(row => row))
                .Concat(o.Table.Caption is null ? [] : [o.Table.Caption])
                .Concat(o.Table.Source is null ? [] : [o.Table.Source])
                .SelectMany(content => content.Lines).SelectMany(line => line.Runs)))
            .SelectMany(Descendants);

    private static IEnumerable<PreviewTextRun> Descendants(PreviewTextRun run)
    {
        yield return run;
        if (run.Footnote is null) yield break;
        foreach (var nested in run.Footnote.Paragraphs.SelectMany(p => p.Lines).SelectMany(l => l.Runs).SelectMany(Descendants))
            yield return nested;
    }
}
