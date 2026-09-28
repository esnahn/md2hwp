using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Md2Hwp.HancomIrPreview;

internal sealed record ProfileStyle(string Symbolic, string NativeName);

internal sealed record ProfileFigureLayout(
    double MaxWidthMillimeters,
    string SourceLabel);

internal sealed record ProfileListLayout(
    int MaxDepth,
    int DepthIndentHwpUnits);

internal sealed record ProfileInsertionTarget(
    string Kind,
    string? Marker);

internal sealed record ProfileBoxSelector(
    string RootStyle,
    string ContentStyle,
    int ContentParagraphs,
    string SourceStyle,
    int SourceParagraphs,
    string PrototypeTextMarker,
    string SourceText);

internal sealed record ProfileCaptionSelector(
    string ParagraphStyle,
    string PrefixBeforeNumber,
    string PrefixAfterNumber,
    string PrototypeCaption)
{
    public int CaptionTextOffset => (PrefixBeforeNumber + PrefixAfterNumber).IndexOf(PrototypeCaption, StringComparison.Ordinal);

    public bool TryReadCaption(string before, string after, out string caption)
    {
        caption = "";
        var slotBefore = PrefixBeforeNumber.Contains(PrototypeCaption, StringComparison.Ordinal);
        var pattern = slotBefore ? PrefixBeforeNumber : PrefixAfterNumber;
        var actual = slotBefore ? before : after;
        if ((slotBefore ? after : before) != (slotBefore ? PrefixAfterNumber : PrefixBeforeNumber)) return false;
        var offset = pattern.IndexOf(PrototypeCaption, StringComparison.Ordinal);
        if (offset < 0) return false;
        var prefix = pattern[..offset];
        var suffix = pattern[(offset + PrototypeCaption.Length)..];
        if (actual.Length <= prefix.Length + suffix.Length ||
            !actual.StartsWith(prefix, StringComparison.Ordinal) || !actual.EndsWith(suffix, StringComparison.Ordinal)) return false;
        caption = actual.Substring(prefix.Length, actual.Length - prefix.Length - suffix.Length);
        return true;
    }
}

internal sealed class InvestigationTemplateProfile
{
    private const int MaximumProfileBytes = 64 * 1024;
    private static readonly Regex SymbolicNamePattern = new(
        "^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$",
        RegexOptions.CultureInvariant | RegexOptions.NonBacktracking);
    private static readonly string[] RequiredParagraphStyles =
    [
        "body",
        "heading.1",
        "heading.2",
        "heading.3",
        "heading.4",
        "heading.5",
        "heading.6",
        "block.box",
        "figure",
        "figure.caption",
        "figure.source",
    ];

    private InvestigationTemplateProfile(
        string id,
        string profilePath,
        string templateSha256,
        long templateBytes,
        IReadOnlyList<ProfileStyle> paragraphStyles,
        string resetNativeStyle,
        ProfileFigureLayout figure,
        ProfileListLayout lists,
        ProfileInsertionTarget insertionTarget,
        ProfileBoxSelector boxSelector,
        ProfileCaptionSelector captionSelector)
    {
        Id = id;
        ProfilePath = profilePath;
        TemplateSha256 = templateSha256;
        TemplateBytes = templateBytes;
        ParagraphStyles = paragraphStyles;
        ResetNativeStyle = resetNativeStyle;
        Figure = figure;
        Lists = lists;
        InsertionTarget = insertionTarget;
        BoxSelector = boxSelector;
        CaptionSelector = captionSelector;
    }

    public string Id { get; }

    public string ProfilePath { get; }

    public string TemplateSha256 { get; }

    public long TemplateBytes { get; }

    public IReadOnlyList<ProfileStyle> ParagraphStyles { get; }

    public string ResetNativeStyle { get; }

    public ProfileFigureLayout Figure { get; }

    public ProfileListLayout Lists { get; }

    public ProfileInsertionTarget InsertionTarget { get; }

    public ProfileBoxSelector BoxSelector { get; }

    public ProfileCaptionSelector CaptionSelector { get; }

    public bool PreserveParagraphLineBreaks { get; private init; }

    // Internal bindings extracted from the actual authored template. No JSON
    // profile is loaded or persisted for this path.
    internal static InvestigationTemplateProfile FromTaggedTemplate(
        string templatePath, IReadOnlyList<ProfileStyle> styles, string reset,
        double width, string sourceLabel, int maxDepth, int indent, ProfileCaptionSelector caption) =>
        new("minimal-tagged-v1", templatePath,
            Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(templatePath))),
            new FileInfo(templatePath).Length, styles, reset,
            new(width, sourceLabel), new(maxDepth, indent),
            new("unique_text_marker", "{{md2hwp:content}}"),
            new("body", "block.box", 1, "figure.source", 1,
                "{{md2hwp:slot:box.content}}", sourceLabel + " "),
            caption)
        { PreserveParagraphLineBreaks = true };

    public void ValidateTemplate(string templatePath)
    {
        var file = new FileInfo(templatePath);
        if (!file.Exists)
        {
            throw new FileNotFoundException("Template file does not exist.", templatePath);
        }
        if (!string.Equals(file.Extension, ".hwp", StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException($"Profile {Id} requires an HWP template: {templatePath}");
        }
        if (file.Length != TemplateBytes)
        {
            throw new InvalidDataException(
                $"Template byte length does not match profile {Id}: expected {TemplateBytes}, got {file.Length}.");
        }
        using var stream = file.OpenRead();
        var hash = Convert.ToHexString(SHA256.HashData(stream));
        if (!string.Equals(hash, TemplateSha256, StringComparison.Ordinal))
        {
            throw new InvalidDataException(
                $"Template SHA-256 does not match profile {Id}: expected {TemplateSha256}, got {hash}.");
        }
    }

    private static ProfileBoxSelector ReadBoxSelector(JsonElement element)
    {
        ProfileJson.ExpectObject(
            element,
            "/selectors/box_prototype",
            [
                "kind",
                "root_style",
                "table_count",
                "content_style",
                "content_paragraphs",
                "source_style",
                "source_paragraphs",
                "prototype_text_marker",
                "source_text",
            ]);
        ProfileJson.ExpectString(element.GetProperty("kind"), "/selectors/box_prototype/kind", "root_inline_table");
        if (ProfileJson.RequiredInt32(element, "table_count", "/selectors/box_prototype") != 1)
        {
            throw ProfileJson.Error("/selectors/box_prototype/table_count", "this closed handler requires exactly one table");
        }
        return new ProfileBoxSelector(
            ProfileJson.RequiredNonemptyString(element, "root_style", "/selectors/box_prototype"),
            ProfileJson.RequiredNonemptyString(element, "content_style", "/selectors/box_prototype"),
            ProfileJson.RequiredInt32(element, "content_paragraphs", "/selectors/box_prototype"),
            ProfileJson.RequiredNonemptyString(element, "source_style", "/selectors/box_prototype"),
            ProfileJson.RequiredInt32(element, "source_paragraphs", "/selectors/box_prototype"),
            ProfileJson.RequiredNonemptyString(element, "prototype_text_marker", "/selectors/box_prototype"),
            ProfileJson.RequiredString(element, "source_text", "/selectors/box_prototype"));
    }

    private static ProfileInsertionTarget ReadInsertionTarget(JsonElement element)
    {
        if (element.ValueKind is not JsonValueKind.Object ||
            !element.TryGetProperty("kind", out var kindElement) ||
            kindElement.ValueKind is not JsonValueKind.String)
        {
            throw ProfileJson.Error(
                "/selectors/insertion_target",
                "expected an object with a string kind");
        }
        var kind = kindElement.GetString();
        if (string.Equals(kind, "document_end", StringComparison.Ordinal))
        {
            ProfileJson.ExpectObject(element, "/selectors/insertion_target", ["kind"]);
            return new ProfileInsertionTarget("document_end", null);
        }
        if (!string.Equals(kind, "unique_text_marker", StringComparison.Ordinal))
        {
            throw ProfileJson.Error(
                "/selectors/insertion_target/kind",
                $"unsupported insertion target {JsonSerializer.Serialize(kind)}");
        }

        ProfileJson.ExpectObject(
            element,
            "/selectors/insertion_target",
            ["kind", "marker", "position", "paragraph"]);
        ProfileJson.ExpectString(
            element.GetProperty("position"),
            "/selectors/insertion_target/position",
            "document_end");
        ProfileJson.ExpectString(
            element.GetProperty("paragraph"),
            "/selectors/insertion_target/paragraph",
            "dedicated_with_empty_successor");
        var marker = ProfileJson.RequiredNonemptyString(
            element,
            "marker",
            "/selectors/insertion_target");
        if (marker.Length > 256 || marker.Contains('\r') || marker.Contains('\n'))
        {
            throw ProfileJson.Error(
                "/selectors/insertion_target/marker",
                "marker must be a CR/LF-free string of at most 256 characters");
        }
        return new ProfileInsertionTarget("unique_text_marker", marker);
    }

    private static string ReadNativeStyleName(
        JsonElement parent,
        string name,
        string path)
    {
        var value = ProfileJson.RequiredNonemptyString(parent, name, path);
        return value.Length <= 128
            ? value
            : throw ProfileJson.Error(path + "/" + name, "native style name exceeds 128 characters");
    }

    private static ProfileCaptionSelector ReadCaptionSelector(JsonElement element)
    {
        ProfileJson.ExpectObject(
            element,
            "/selectors/caption_prototype",
            [
                "kind",
                "paragraph_style",
                "prefix_before_number",
                "prefix_after_number",
                "prototype_caption",
                "number_type",
                "number_format",
            ]);
        ProfileJson.ExpectString(
            element.GetProperty("kind"),
            "/selectors/caption_prototype/kind",
            "root_figure_autonum");
        ProfileJson.ExpectString(
            element.GetProperty("number_type"),
            "/selectors/caption_prototype/number_type",
            "Figure");
        ProfileJson.ExpectString(
            element.GetProperty("number_format"),
            "/selectors/caption_prototype/number_format",
            "Digit");
        return new ProfileCaptionSelector(
            ProfileJson.RequiredNonemptyString(element, "paragraph_style", "/selectors/caption_prototype"),
            ProfileJson.RequiredNonemptyString(element, "prefix_before_number", "/selectors/caption_prototype"),
            ProfileJson.RequiredNonemptyString(element, "prefix_after_number", "/selectors/caption_prototype"),
            ProfileJson.RequiredNonemptyString(element, "prototype_caption", "/selectors/caption_prototype"));
    }

    private void ValidateSelectors()
    {
        var symbolicStyles = ParagraphStyles
            .Select(style => style.Symbolic)
            .ToHashSet(StringComparer.Ordinal);
        foreach (var (path, symbolic) in new[]
        {
            ("/selectors/box_prototype/root_style", BoxSelector.RootStyle),
            ("/selectors/box_prototype/content_style", BoxSelector.ContentStyle),
            ("/selectors/box_prototype/source_style", BoxSelector.SourceStyle),
            ("/selectors/caption_prototype/paragraph_style", CaptionSelector.ParagraphStyle),
        })
        {
            if (!symbolicStyles.Contains(symbolic))
            {
                throw ProfileJson.Error(path, $"unmapped symbolic paragraph style {symbolic}");
            }
        }
        if (BoxSelector.ContentParagraphs != 1 || BoxSelector.SourceParagraphs != 1)
        {
            throw ProfileJson.Error(
                "/selectors/box_prototype",
                "the minimal-fixture box handler requires one content and one source paragraph");
        }
    }
}

internal static class ProfileJson
{
    public static void ValidateNoDuplicateMembers(JsonElement element, string path)
    {
        if (element.ValueKind is JsonValueKind.Object)
        {
            var names = new HashSet<string>(StringComparer.Ordinal);
            foreach (var property in element.EnumerateObject())
            {
                if (!names.Add(property.Name))
                {
                    throw Error(path, $"duplicate object member {property.Name}");
                }
                ValidateNoDuplicateMembers(property.Value, path + "/" + property.Name);
            }
        }
        else if (element.ValueKind is JsonValueKind.Array)
        {
            var index = 0;
            foreach (var item in element.EnumerateArray())
            {
                ValidateNoDuplicateMembers(item, $"{path}/{index}");
                index++;
            }
        }
    }

    public static void ExpectObject(
        JsonElement element,
        string path,
        IReadOnlyCollection<string> required)
    {
        if (element.ValueKind is not JsonValueKind.Object)
        {
            throw Error(path, "expected object");
        }
        var allowed = new HashSet<string>(required, StringComparer.Ordinal);
        foreach (var property in element.EnumerateObject())
        {
            if (!allowed.Contains(property.Name))
            {
                throw Error(path + "/" + property.Name, "unknown object member");
            }
        }
        foreach (var property in required)
        {
            if (!element.TryGetProperty(property, out _))
            {
                throw Error(path + "/" + property, "missing required object member");
            }
        }
    }

    public static string RequiredNonemptyString(JsonElement parent, string name, string path)
    {
        var value = RequiredString(parent, name, path);
        return value.Length > 0
            ? value
            : throw Error(path + "/" + name, "expected nonempty string");
    }

    public static string RequiredString(JsonElement parent, string name, string path)
    {
        var element = parent.GetProperty(name);
        return element.ValueKind is JsonValueKind.String
            ? element.GetString()!
            : throw Error(path + "/" + name, "expected string");
    }

    public static int RequiredInt32(JsonElement parent, string name, string path)
    {
        var element = parent.GetProperty(name);
        return element.ValueKind is JsonValueKind.Number && element.TryGetInt32(out var value)
            ? value
            : throw Error(path + "/" + name, "expected 32-bit integer");
    }

    public static long RequiredInt64(JsonElement parent, string name, string path)
    {
        var element = parent.GetProperty(name);
        return element.ValueKind is JsonValueKind.Number && element.TryGetInt64(out var value)
            ? value
            : throw Error(path + "/" + name, "expected 64-bit integer");
    }

    public static double RequiredDouble(JsonElement parent, string name, string path)
    {
        var element = parent.GetProperty(name);
        return element.ValueKind is JsonValueKind.Number && element.TryGetDouble(out var value)
            ? value
            : throw Error(path + "/" + name, "expected number");
    }

    public static void ExpectString(JsonElement element, string path, string expected)
    {
        if (element.ValueKind is not JsonValueKind.String ||
            !string.Equals(element.GetString(), expected, StringComparison.Ordinal))
        {
            throw Error(path, $"expected {expected}");
        }
    }

    public static void ExpectTrue(JsonElement element, string path)
    {
        if (element.ValueKind is not JsonValueKind.True)
        {
            throw Error(path, "expected true");
        }
    }

    public static InvalidDataException Error(string path, string message) =>
        new($"invalid template profile at {(path.Length == 0 ? "/" : path)}: {message}");
}
