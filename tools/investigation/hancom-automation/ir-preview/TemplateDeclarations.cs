namespace Md2Hwp.HancomIrPreview;

// Lexical experiment only. Paragraph indices are not native COM coordinates.
internal sealed record DeclarationRange(int StartInclusive, int EndExclusive,
    IReadOnlyDictionary<string, int> Slots);

internal sealed record TemplateDeclarationPlan(int ContentParagraph, int SamplesBegin,
    int SamplesEnd, IReadOnlyDictionary<string, int> Styles,
    IReadOnlyDictionary<string, DeclarationRange> Prototypes);

internal static class TemplateDeclarations
{
    private const string Prefix = "{{md2hwp:";
    private static readonly Dictionary<string, string[]> RequiredSlots = new(StringComparer.Ordinal)
    {
        ["block.box"] = ["box.content"],
        ["figure"] = ["figure.image", "figure.caption", "figure.source"],
        ["list.bullet"] = ["list.item"],
        ["list.ordered"] = ["list.item"],
    };

    public static TemplateDeclarationPlan Parse(IReadOnlyList<string> paragraphs)
    {
        int content = -1, samplesBegin = -1, samplesEnd = -1, contract = -1;
        string? owner = null;
        int begin = -1;
        var styles = new Dictionary<string, int>(StringComparer.Ordinal);
        var prototypes = new Dictionary<string, DeclarationRange>(StringComparer.Ordinal);
        var slots = new Dictionary<string, int>(StringComparer.Ordinal);

        for (var i = 0; i < paragraphs.Count; i++)
        {
            var text = paragraphs[i];
            if (!text.Contains(Prefix, StringComparison.Ordinal)) continue;
            if (!text.StartsWith(Prefix, StringComparison.Ordinal) ||
                !text.EndsWith("}}", StringComparison.Ordinal) ||
                text.Contains('\r') || text.Contains('\n'))
                throw Error(i, "declaration must occupy a whole paragraph");
            var tag = text[Prefix.Length..^2];
            var inSamples = samplesBegin >= 0 && samplesEnd < 0;
            if (tag == "content")
            {
                if (content >= 0 || inSamples) throw Error(i, "duplicate or scoped content target");
                content = i;
            }
            else if (tag == "begin:samples")
            {
                if (samplesBegin >= 0) throw Error(i, "duplicate samples range");
                samplesBegin = i;
            }
            else if (tag == "end:samples")
            {
                if (!inSamples || owner is not null) throw Error(i, "unmatched or crossed samples end");
                samplesEnd = i;
            }
            else
            {
                if (!inSamples) throw Error(i, "declaration outside samples range");
                if (tag == "contract:experimental-1")
                {
                    if (contract >= 0 || owner is not null) throw Error(i, "duplicate or scoped contract");
                    contract = i;
                }
                else if (tag == "body" || IsHeading(tag))
                {
                    if (owner is not null || !styles.TryAdd(tag, i))
                        throw Error(i, "duplicate or scoped paragraph style");
                }
                else if (tag.StartsWith("begin:", StringComparison.Ordinal))
                {
                    var role = tag[6..];
                    if (owner is not null || !RequiredSlots.ContainsKey(role) || prototypes.ContainsKey(role))
                        throw Error(i, "nested, duplicate, or unknown prototype");
                    owner = role;
                    begin = i;
                    slots = new(StringComparer.Ordinal);
                }
                else if (tag.StartsWith("end:", StringComparison.Ordinal))
                {
                    if (owner is null || tag[4..] != owner) throw Error(i, "unmatched or crossed prototype end");
                    if (i == begin + 1) throw Error(i, "empty prototype");
                    var required = RequiredSlots[owner];
                    foreach (var slot in required)
                        if (!slots.ContainsKey(slot)) throw Error(i, $"missing slot {slot}");
                    if (owner == "figure" && !(slots[required[0]] < slots[required[1]] &&
                                               slots[required[1]] < slots[required[2]]))
                        throw Error(i, "figure slots out of order");
                    prototypes.Add(owner, new(begin + 1, i, slots));
                    owner = null;
                }
                else if (tag.StartsWith("slot:", StringComparison.Ordinal))
                {
                    var slot = tag[5..];
                    if (owner is null || !RequiredSlots[owner].Contains(slot, StringComparer.Ordinal) ||
                        !slots.TryAdd(slot, i)) throw Error(i, "orphan, wrong-owner, or duplicate slot");
                }
                else throw Error(i, $"unknown declaration {tag}");
            }
        }

        if (content < 0 || samplesBegin < 0 || samplesEnd < 0 || contract < 0 ||
            !styles.ContainsKey("body") || owner is not null)
            throw Error(paragraphs.Count, "missing required declaration or unclosed range");
        if (samplesEnd == paragraphs.Count - 1 || content == paragraphs.Count - 1)
            throw Error(paragraphs.Count, "removable paragraph range requires a successor");
        return new(content, samplesBegin, samplesEnd, styles, prototypes);
    }

    private static bool IsHeading(string tag) => tag.Length == 9 &&
        tag.StartsWith("heading.", StringComparison.Ordinal) && tag[8] is >= '1' and <= '6';

    private static InvalidDataException Error(int index, string message) =>
        new($"template declaration at paragraph[{index}]: {message}");
}
