using System.Text.Json;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record PreviewSourceParagraph(string Prefix, PreviewInlineContent Content)
{
    private static readonly Regex PrefixPattern = ReadPattern();
    private static Regex ReadPattern()
    {
        using var stream = typeof(IrContract).Assembly.GetManifestResourceStream("md2hwp.ir.schema")!;
        using var schema = JsonDocument.Parse(stream);
        var pattern = schema.RootElement.GetProperty("$defs").GetProperty("source_paragraph")
            .GetProperty("properties").GetProperty("prefix").GetProperty("pattern").GetString()!;
        return new Regex(pattern, RegexOptions.CultureInvariant);
    }
    internal static IReadOnlyList<PreviewSourceParagraph>? ReadOptional(JsonElement block, string path)
    {
        if (!block.TryGetProperty("source", out var value) || value.ValueKind == JsonValueKind.Null) return null;
        var array = JsonContract.ExpectArray(value, path + "/source");
        if (array.GetArrayLength() == 0) throw JsonContract.Error(path + "/source", "source paragraphs must not be empty");
        return array.EnumerateArray().Select((paragraph, i) => {
            var p = $"{path}/source/{i}";
            JsonContract.ExpectObject(paragraph, p, ["prefix", "inlines"]);
            var prefix = JsonContract.RequiredString(paragraph, "prefix", p);
            // Regex '$' can match before a final newline: require a full match.
            var match = PrefixPattern.Match(prefix);
            if (!match.Success || match.Length != prefix.Length)
                throw JsonContract.Error(p + "/prefix", "unrecognized source prefix or invalid punctuation");
            var content = InlineText.Read(paragraph.GetProperty("inlines"), p + "/inlines", allowFootnotes: false, allowCrossReferences: false);
            if (string.IsNullOrWhiteSpace(content.Flatten(" ").Text))
                throw JsonContract.Error(p + "/inlines", "source paragraph content must not be empty");
            return new PreviewSourceParagraph(prefix, content);
        }).ToArray();
    }
}

// Clone the template's source paragraph for every IR note after the verified
// one-paragraph intermediate has been attached to native objects.
internal sealed class TemplateObjectSources(XDocument source, IReadOnlyDictionary<string, XElement> samples)
{
    internal static string PrefixSlot(string role) => TaggedTemplateBinding.Tag("slot:" + role + ".source.prefix");
    internal static (TemplateObjectSources Layout, XDocument Document) Lower(XDocument source)
    {
        var document = new XDocument(source);
        var samples = new Dictionary<string, XElement>(StringComparer.Ordinal);
        foreach (var role in new[] { "code", "figure", "table" })
        {
            var contentSlot = TaggedTemplateBinding.Tag("slot:" + role + ".source");
            var prefixSlot = PrefixSlot(role);
            var paragraphs = document.Descendants("P").Where(p => TaggedTemplateBinding.DirectText(p).Contains(contentSlot, StringComparison.Ordinal)).ToArray();
            if (paragraphs.Length != 1)
                throw new InvalidDataException($"{role}.source requires one source sample. Regenerate with init-template for IR 0.4.");
            var paragraph = paragraphs[0];
            var text = TaggedTemplateBinding.DirectText(paragraph);
            if (text.Split(prefixSlot, StringSplitOptions.None).Length != 2 ||
                document.Descendants("P").Count(p => TaggedTemplateBinding.DirectText(p).Contains(prefixSlot, StringComparison.Ordinal)) != 1 ||
                !text.Contains(prefixSlot + ": " + contentSlot, StringComparison.Ordinal))
                throw new InvalidDataException($"{role}.source requires {prefixSlot}: {contentSlot} in one plain paragraph. Adopt the prefix slot or regenerate for IR 0.4.");
            if (paragraph.Elements().Any(e => e.Name.LocalName != "TEXT") ||
                paragraph.Elements("TEXT").SelectMany(t => t.Elements()).Any(e => e.Name.LocalName != "CHAR" || e.HasElements))
                throw new InvalidDataException("Source samples require plain text without controls.");
            var remaining = text.Replace(prefixSlot, "", StringComparison.Ordinal).Replace(contentSlot, "", StringComparison.Ordinal);
            if (remaining.Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
                throw new InvalidDataException("Unsupported tag in source sample.");
            samples.Add(role, new XElement(paragraph));
            var transformed = TemplateMetadata.Transform(new XDocument(new XElement("ROOT", new XElement(paragraph))),
                prefixSlot, new Regex(Regex.Escape(prefixSlot), RegexOptions.CultureInvariant), _ => "출처");
            paragraph.ReplaceNodes(transformed.Root!.Element("P")!.Nodes().ToArray());
        }
        return (new(new XDocument(source), samples), document);
    }

    internal XDocument Attach(XDocument rendered, IrPreviewPlan plan, int start)
    {
        var result = new XDocument(rendered);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(result);
        var index = start;
        foreach (var operation in plan.Operations)
        {
            if (operation.Sources is { } paragraphs)
            {
                var role = operation.Kind == "code" ? "code" : operation.Kind;
                var root = roots[index];
                XElement list;
                if (role == "table")
                    list = root.Descendants("TABLE").Single().Elements("ROW").Last().Element("CELL")!.Element("PARALIST")!;
                else
                    list = root.Descendants(role == "code" ? "TABLE" : "PICTURE").Single()
                        .Element("SHAPEOBJECT")!.Element("CAPTION")!.Element("PARALIST")!;
                var generated = paragraphs.Select(p => {
                    var sample = TemplateHeadingBlocks.ImportParagraph(samples[role], source, result);
                    TemplateTables.FillSlot(sample, PrefixSlot(role), PreviewInlineContent.Plain([p.Prefix]), result);
                    TemplateTables.FillSlot(sample, TaggedTemplateBinding.Tag("slot:" + role + ".source"), p.Content, result);
                    return sample;
                }).ToArray();
                if (role == "figure")
                {
                    var title = list.Elements("P").First();
                    list.ReplaceNodes(new[] { new XElement(title) }.Concat(generated));
                }
                else list.ReplaceNodes(generated);
            }
            index++;
        }
        return result;
    }
}
