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
    string PrototypeCaption);

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

    public static InvestigationTemplateProfile Load(
        string profilePath,
        string repositoryRoot)
    {
        var fullPath = Path.GetFullPath(profilePath);
        var profilesRoot = Path.Combine(repositoryRoot, "profiles", "templates");
        if (!PathSafety.IsWithin(profilesRoot, fullPath))
        {
            throw new InvalidDataException(
                $"Investigation template profile must be under {profilesRoot}: {fullPath}");
        }
        var bytes = File.ReadAllBytes(fullPath);
        if (bytes.Length == 0 || bytes.Length > MaximumProfileBytes)
        {
            throw new InvalidDataException(
                $"Template profile must be 1 through {MaximumProfileBytes} bytes: {fullPath}");
        }

        var json = new UTF8Encoding(false, true).GetString(bytes);
        using var document = JsonDocument.Parse(json, new JsonDocumentOptions
        {
            AllowTrailingCommas = false,
            CommentHandling = JsonCommentHandling.Disallow,
            MaxDepth = 32,
        });
        ProfileJson.ValidateNoDuplicateMembers(document.RootElement, "");
        var root = document.RootElement;
        ProfileJson.ExpectObject(
            root,
            "",
            [
                "schema",
                "profile_version",
                "id",
                "status",
                "target_ir",
                "backend",
                "template",
                "styles",
                "selectors",
                "layout",
                "capabilities",
            ]);
        ProfileJson.ExpectString(root.GetProperty("schema"), "/schema", "md2hwp.template-profile");
        ProfileJson.ExpectString(root.GetProperty("profile_version"), "/profile_version", "0.1");
        ProfileJson.ExpectString(root.GetProperty("status"), "/status", "investigation");
        ProfileJson.ExpectString(root.GetProperty("backend"), "/backend", "hancom-automation");
        var id = ProfileJson.RequiredNonemptyString(root, "id", "");
        if (!SymbolicNamePattern.IsMatch(id))
        {
            throw ProfileJson.Error("/id", "expected a symbolic profile name");
        }

        var targetIr = root.GetProperty("target_ir");
        ProfileJson.ExpectObject(targetIr, "/target_ir", ["schema", "version"]);
        ProfileJson.ExpectString(targetIr.GetProperty("schema"), "/target_ir/schema", "md2hwp.ir");
        ProfileJson.ExpectString(targetIr.GetProperty("version"), "/target_ir/version", "0.1");

        var template = root.GetProperty("template");
        ProfileJson.ExpectObject(template, "/template", ["format", "identity"]);
        ProfileJson.ExpectString(template.GetProperty("format"), "/template/format", "hwp");
        var identity = template.GetProperty("identity");
        ProfileJson.ExpectObject(identity, "/template/identity", ["kind", "sha256", "bytes"]);
        ProfileJson.ExpectString(identity.GetProperty("kind"), "/template/identity/kind", "exact_sha256");
        var templateSha256 = ProfileJson.RequiredNonemptyString(identity, "sha256", "/template/identity");
        if (templateSha256.Length != 64 ||
            templateSha256.Any(character =>
                character is not (>= '0' and <= '9' or >= 'A' and <= 'F')))
        {
            throw ProfileJson.Error("/template/identity/sha256", "expected 64 uppercase hexadecimal characters");
        }
        var templateBytes = ProfileJson.RequiredInt64(identity, "bytes", "/template/identity");
        if (templateBytes <= 0)
        {
            throw ProfileJson.Error("/template/identity/bytes", "expected a positive byte length");
        }

        var styles = root.GetProperty("styles");
        ProfileJson.ExpectObject(styles, "/styles", ["paragraphs", "reset"]);
        var paragraphs = styles.GetProperty("paragraphs");
        ProfileJson.ExpectObject(paragraphs, "/styles/paragraphs", RequiredParagraphStyles);
        var paragraphStyles = RequiredParagraphStyles
            .Select(symbolic => new ProfileStyle(
                symbolic,
                ReadNativeStyleName(paragraphs, symbolic, "/styles/paragraphs")))
            .ToArray();
        var resetStyle = ReadNativeStyleName(styles, "reset", "/styles");
        if (string.Equals(
                resetStyle,
                paragraphStyles.Single(style => style.Symbolic == "body").NativeName,
                StringComparison.Ordinal))
        {
            throw ProfileJson.Error(
                "/styles/reset",
                "reset style must differ from the body style for the HWP style detour");
        }

        var selectors = root.GetProperty("selectors");
        ProfileJson.ExpectObject(
            selectors,
            "/selectors",
            ["insertion_target", "box_prototype", "caption_prototype"]);
        var insertionTarget = selectors.GetProperty("insertion_target");
        var parsedInsertionTarget = ReadInsertionTarget(insertionTarget);
        var boxSelector = ReadBoxSelector(selectors.GetProperty("box_prototype"));
        var captionSelector = ReadCaptionSelector(selectors.GetProperty("caption_prototype"));

        var layout = root.GetProperty("layout");
        ProfileJson.ExpectObject(layout, "/layout", ["figure", "lists"]);
        var figure = layout.GetProperty("figure");
        ProfileJson.ExpectObject(
            figure,
            "/layout/figure",
            ["max_width_mm", "placement", "source_line", "source_label"]);
        var maxWidth = ProfileJson.RequiredDouble(figure, "max_width_mm", "/layout/figure");
        if (!double.IsFinite(maxWidth) || maxWidth <= 0)
        {
            throw ProfileJson.Error("/layout/figure/max_width_mm", "expected a positive finite number");
        }
        ProfileJson.ExpectString(figure.GetProperty("placement"), "/layout/figure/placement", "inline_character");
        ProfileJson.ExpectString(
            figure.GetProperty("source_line"),
            "/layout/figure/source_line",
            "immediately_after_caption");

        var lists = layout.GetProperty("lists");
        ProfileJson.ExpectObject(
            lists,
            "/layout/lists",
            [
                "body_style",
                "max_depth",
                "depth_indent_hwp_units",
                "ordered_number_format",
                "ordered_delimiter",
            ]);
        ProfileJson.ExpectString(lists.GetProperty("body_style"), "/layout/lists/body_style", "body");
        ProfileJson.ExpectString(
            lists.GetProperty("ordered_number_format"),
            "/layout/lists/ordered_number_format",
            "decimal");
        ProfileJson.ExpectString(
            lists.GetProperty("ordered_delimiter"),
            "/layout/lists/ordered_delimiter",
            "period");
        var maxDepth = ProfileJson.RequiredInt32(lists, "max_depth", "/layout/lists");
        if (maxDepth is < 0 or > 6)
        {
            throw ProfileJson.Error("/layout/lists/max_depth", "expected an integer from 0 through 6");
        }
        var depthIndent = ProfileJson.RequiredInt32(
            lists,
            "depth_indent_hwp_units",
            "/layout/lists");
        if (depthIndent < 0)
        {
            throw ProfileJson.Error("/layout/lists/depth_indent_hwp_units", "expected a nonnegative integer");
        }

        var capabilities = root.GetProperty("capabilities");
        var capabilityNames = new[]
        {
            "native_bullets",
            "native_numbering",
            "inline_pictures",
            "figure_autonum",
            "saveblock_insertion",
        };
        ProfileJson.ExpectObject(capabilities, "/capabilities", capabilityNames);
        foreach (var capability in capabilityNames)
        {
            ProfileJson.ExpectTrue(capabilities.GetProperty(capability), "/capabilities/" + capability);
        }

        var result = new InvestigationTemplateProfile(
            id,
            fullPath,
            templateSha256,
            templateBytes,
            paragraphStyles,
            resetStyle,
            new ProfileFigureLayout(
                maxWidth,
                ProfileJson.RequiredNonemptyString(figure, "source_label", "/layout/figure")),
            new ProfileListLayout(maxDepth, depthIndent),
            parsedInsertionTarget,
            boxSelector,
            captionSelector);
        result.ValidateSelectors();
        return result;
    }

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
