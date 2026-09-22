using System.Buffers.Binary;
using System.Text;
using System.Text.Json;

namespace Md2Hwp.HancomIrPreview;

internal sealed record IrPreviewPlan(
    string IrPath,
    string ProfileId,
    PreviewSummary Summary,
    IReadOnlyList<PreviewOperation> Operations,
    IReadOnlyList<string> Limitations)
{
    public static IrPreviewPlan Load(
        string irPath,
        string repositoryRoot,
        InvestigationTemplateProfile profile)
    {
        if (!File.Exists(irPath))
        {
            throw new FileNotFoundException("Missing IR input.", irPath);
        }

        var bytes = File.ReadAllBytes(irPath);
        if (bytes.Length > 8 * 1024 * 1024)
        {
            throw new InvalidDataException("The investigation preview limits IR input to 8 MiB.");
        }

        using var document = JsonDocument.Parse(bytes, new JsonDocumentOptions
        {
            AllowTrailingCommas = false,
            CommentHandling = JsonCommentHandling.Disallow,
            MaxDepth = 64,
        });
        JsonContract.ValidateNoDuplicateMembers(document.RootElement, string.Empty);
        JsonContract.ExpectObject(
            document.RootElement,
            string.Empty,
            ["schema", "ir_version", "metadata", "blocks"]);
        JsonContract.ExpectString(document.RootElement.GetProperty("schema"), "/schema", "md2hwp.ir");
        JsonContract.ExpectString(document.RootElement.GetProperty("ir_version"), "/ir_version", "0.1");

        var metadata = document.RootElement.GetProperty("metadata");
        JsonContract.ExpectObject(metadata, "/metadata", []);
        var blocks = JsonContract.ExpectArray(document.RootElement.GetProperty("blocks"), "/blocks");

        var builder = new PlanBuilder(
            Path.GetFullPath(irPath),
            Path.GetFullPath(repositoryRoot),
            profile);
        var index = 0;
        foreach (var block in blocks.EnumerateArray())
        {
            builder.AddBlock(block, $"/blocks/{index}");
            index++;
        }
        return builder.Build(index);
    }
}

internal sealed record PreviewSummary(
    int SourceBlocks,
    int TextOperations,
    int BoxOperations,
    int FigureOperations,
    int ListItems);

internal sealed record PreviewOperation(
    string Kind,
    string Label,
    IReadOnlyList<string> Lines,
    string? ImagePath = null,
    double? ImageWidthMillimeters = null,
    double? ImageHeightMillimeters = null,
    string? ParagraphStyle = null,
    IReadOnlyList<IReadOnlyList<PreviewTextRun>>? FormattedLines = null,
    PreviewListMarker? ListMarker = null);

internal sealed record PreviewListMarker(
    int ListId,
    string Kind,
    int Depth,
    int Start,
    int Number,
    bool StartsList);

internal sealed record PreviewTextRun(
    string Text,
    bool Strong,
    bool Emphasis);

internal sealed record PreviewLine(
    string Text,
    IReadOnlyList<PreviewTextRun> Runs);

internal sealed record PreviewInlineContent(
    IReadOnlyList<PreviewLine> Lines)
{
    public static PreviewInlineContent Plain(IReadOnlyList<string> lines) =>
        new(lines.Select(line => new PreviewLine(
            line,
            line.Length == 0 ? [] : [new PreviewTextRun(line, false, false)])).ToArray());

    public PreviewLine Flatten(string separator)
    {
        var runs = new List<PreviewTextRun>();
        for (var index = 0; index < Lines.Count; index++)
        {
            if (index > 0)
            {
                runs.Add(new PreviewTextRun(separator, false, false));
            }
            runs.AddRange(Lines[index].Runs);
        }
        return new PreviewLine(
            string.Join(separator, Lines.Select(line => line.Text)),
            PreviewRunBuilder.Coalesce(runs));
    }
}

internal sealed class PlanBuilder(
    string irPath,
    string repositoryRoot,
    InvestigationTemplateProfile profile)
{
    private readonly List<PreviewOperation> operations = [];
    private int listItems;
    private int nextListId;

    public void AddBlock(JsonElement block, string path)
    {
        if (block.ValueKind is not JsonValueKind.Object)
        {
            throw JsonContract.Error(path, "block must be an object");
        }
        var type = JsonContract.RequiredString(block, "type", path);
        switch (type)
        {
            case "heading":
                AddHeading(block, path);
                break;
            case "paragraph":
                AddParagraph(block, path, null);
                break;
            case "verbatim_block":
                AddVerbatimBlock(block, path);
                break;
            case "list":
                AddList(block, path, 0);
                break;
            case "figure":
                AddFigure(block, path);
                break;
            default:
                throw JsonContract.Error(path + "/type", $"unsupported IR block type {type}");
        }
    }

    public IrPreviewPlan Build(int sourceBlocks)
    {
        var textOperations = operations.Count(operation => operation.Kind == "text");
        var boxOperations = operations.Count(operation => operation.Kind == "box");
        var figureOperations = operations.Count(operation => operation.Kind == "figure");
        return new IrPreviewPlan(
            irPath,
            profile.Id,
            new PreviewSummary(
                sourceBlocks,
                textOperations,
                boxOperations,
                figureOperations,
                listItems),
            operations,
            [
                "This is an investigation preview, not backend lowering.",
                "AURI paragraph styles are bound by unique native names during render.",
                "Strong/emphasis marks are retained as character-shape runs; link targets remain flattened.",
                "verbatim_block maps to one prototype-backed block.box operation; render accepts only the uniquely matched minimal-fixture box structure.",
                "The minimal-fixture box prototype's source placeholder remains template decoration; its production metadata contract is unresolved.",
                "Figure captions remain one prototype-backed native AUTONUM operation; render accepts only the uniquely matched minimal-fixture root-caption structure.",
                profile.PreserveParagraphLineBreaks
                    ? "IR line_break nodes remain native line breaks in the same paragraph."
                    : "IR line_break nodes outside verbatim blocks are previewed as separate HWP paragraphs.",
                $"Only trusted repository-local PNG figures are inserted; width is limited to {profile.Figure.MaxWidthMillimeters} mm by the investigation profile and aspect ratio is preserved.",
            ]);
    }

    private void AddHeading(JsonElement block, string path)
    {
        JsonContract.ExpectObject(block, path, ["type", "level", "inlines"]);
        var level = JsonContract.RequiredInt32(block, "level", path);
        if (level is < 1 or > 6)
        {
            throw JsonContract.Error(path + "/level", "heading level must be between 1 and 6");
        }
        var style = $"heading.{level}";
        operations.Add(Text(style, style, InlineText.Read(block.GetProperty("inlines"), path + "/inlines")));
    }

    private void AddParagraph(JsonElement block, string path, string? listLabel)
    {
        AddParagraph(block, path, listLabel, null);
    }

    private void AddParagraph(
        JsonElement block,
        string path,
        string? listLabel,
        PreviewListMarker? listMarker)
    {
        JsonContract.ExpectObject(block, path, ["type", "inlines"]);
        const string style = "body";
        var label = listLabel ?? "body";
        var content = InlineText.Read(block.GetProperty("inlines"), path + "/inlines");
        if (listMarker is not null && content.Lines.Count != 1 && !profile.PreserveParagraphLineBreaks)
        {
            throw JsonContract.Error(
                path + "/inlines",
                "the native-list investigation preview does not support line_break in a list marker paragraph");
        }
        operations.Add(Text(label, style, content, listMarker));
    }

    private void AddVerbatimBlock(JsonElement block, string path)
    {
        JsonContract.ExpectObject(block, path, ["type", "lines"]);
        var linesElement = JsonContract.ExpectArray(block.GetProperty("lines"), path + "/lines");
        if (linesElement.GetArrayLength() == 0)
        {
            throw JsonContract.Error(path + "/lines", "verbatim block must contain at least one line");
        }
        var lines = new List<string>();
        var index = 0;
        foreach (var line in linesElement.EnumerateArray())
        {
            var value = JsonContract.ReadString(line, $"{path}/lines/{index}");
            if (value.Contains('\r') || value.Contains('\n'))
            {
                throw JsonContract.Error(
                    $"{path}/lines/{index}",
                    "verbatim block line must not contain CR or LF");
            }
            lines.Add(value);
            index++;
        }
        var content = PreviewInlineContent.Plain(lines);
        operations.Add(new PreviewOperation(
            "box",
            "verbatim_block",
            content.Lines.Select(line => line.Text).ToArray(),
            ParagraphStyle: "block.box",
            FormattedLines: content.Lines.Select(line => line.Runs).ToArray()));
    }

    private void AddList(JsonElement block, string path, int depth)
    {
        if (depth > profile.Lists.MaxDepth)
        {
            throw JsonContract.Error(
                path,
                $"template profile {profile.Id} supports list depths 0 through {profile.Lists.MaxDepth}");
        }
        JsonContract.ExpectObject(block, path, ["type", "kind", "tight", "items"], ["start"]);
        var kind = JsonContract.RequiredString(block, "kind", path);
        if (kind is not ("bullet" or "ordered"))
        {
            throw JsonContract.Error(path + "/kind", "list kind must be bullet or ordered");
        }
        _ = JsonContract.RequiredBoolean(block, "tight", path);
        var nextNumber = 1;
        if (kind == "ordered")
        {
            nextNumber = block.TryGetProperty("start", out var start)
                ? JsonContract.ReadInt32(start, path + "/start")
                : throw JsonContract.Error(path + "/start", "ordered list requires start");
        }
        else if (block.TryGetProperty("start", out _))
        {
            throw JsonContract.Error(path + "/start", "bullet list must not contain start");
        }

        var items = JsonContract.ExpectArray(block.GetProperty("items"), path + "/items");
        var listId = nextListId++;
        var listStart = nextNumber;
        var itemIndex = 0;
        foreach (var item in items.EnumerateArray())
        {
            var itemPath = $"{path}/items/{itemIndex}";
            JsonContract.ExpectObject(item, itemPath, ["blocks"]);
            var itemNumber = kind == "ordered" ? nextNumber++ : itemIndex + 1;
            var label = $"list.{kind}.depth-{depth}.item-{itemNumber}";
            listItems++;
            var markerPending = true;

            var itemBlocks = JsonContract.ExpectArray(item.GetProperty("blocks"), itemPath + "/blocks");
            var blockIndex = 0;
            foreach (var itemBlock in itemBlocks.EnumerateArray())
            {
                var blockPath = $"{itemPath}/blocks/{blockIndex}";
                var blockType = JsonContract.RequiredString(itemBlock, "type", blockPath);
                switch (blockType)
                {
                    case "paragraph":
                        AddParagraph(
                            itemBlock,
                            blockPath,
                            label,
                            markerPending
                                ? new PreviewListMarker(
                                    listId,
                                    kind,
                                    depth,
                                    listStart,
                                    itemNumber,
                                    itemIndex == 0)
                                : null);
                        markerPending = false;
                        break;
                    case "list":
                        AddList(itemBlock, blockPath, depth + 1);
                        break;
                    default:
                        throw JsonContract.Error(blockPath + "/type", "list items may contain only paragraph or list blocks");
                }
                blockIndex++;
            }
            if (markerPending)
            {
                throw JsonContract.Error(
                    itemPath + "/blocks",
                    "the native-list investigation preview requires a paragraph to carry each item marker");
            }
            itemIndex++;
        }
    }

    private void AddFigure(JsonElement block, string path)
    {
        JsonContract.ExpectObject(block, path, ["type", "image", "caption", "source"]);
        var image = block.GetProperty("image");
        JsonContract.ExpectObject(image, path + "/image", ["path", "alt", "title"]);
        var relativePath = JsonContract.RequiredString(image, "path", path + "/image");
        if (Path.IsPathRooted(relativePath) || relativePath.Contains('\\'))
        {
            throw JsonContract.Error(path + "/image/path", "image path must be a canonical relative path using forward slashes");
        }
        JsonContract.ExpectNullOrString(image.GetProperty("title"), path + "/image/title");

        var imagePath = Path.GetFullPath(Path.Combine(
            Path.GetDirectoryName(irPath)!,
            relativePath.Replace('/', Path.DirectorySeparatorChar)));
        if (!PathSafety.IsWithin(repositoryRoot, imagePath) || !File.Exists(imagePath))
        {
            throw JsonContract.Error(path + "/image/path", "image must resolve to an existing file inside the repository");
        }
        if (!string.Equals(Path.GetExtension(imagePath), ".png", StringComparison.OrdinalIgnoreCase))
        {
            throw JsonContract.Error(path + "/image/path", "the investigation preview supports PNG figures only");
        }

        var (pixelWidth, pixelHeight) = PngDimensions.Read(imagePath);
        var width = profile.Figure.MaxWidthMillimeters;
        var height = width * pixelHeight / pixelWidth;
        var alt = InlineText.Read(image.GetProperty("alt"), path + "/image/alt");
        var caption = InlineText.Read(block.GetProperty("caption"), path + "/caption");
        var sourceElement = block.GetProperty("source");
        var source = sourceElement.ValueKind is JsonValueKind.Null
            ? PreviewInlineContent.Plain([string.Empty])
            : InlineText.Read(sourceElement, path + "/source");
        var figureLines = new[]
        {
            alt.Flatten(" / "),
            caption.Flatten(" / "),
            source.Flatten(" / "),
        };
        operations.Add(new PreviewOperation(
            "figure",
            "figure",
            figureLines.Select(line => line.Text).ToArray(),
            imagePath,
            width,
            height,
            "body",
            figureLines.Select(line => line.Runs).ToArray()));
    }

    private static PreviewOperation Text(
        string label,
        string paragraphStyle,
        PreviewInlineContent content,
        PreviewListMarker? listMarker = null) =>
        new(
            "text",
            label,
            content.Lines.Select(line => line.Text).ToArray(),
            ParagraphStyle: paragraphStyle,
            FormattedLines: content.Lines.Select(line => line.Runs).ToArray(),
            ListMarker: listMarker);
}

internal static class InlineText
{
    public static PreviewInlineContent Read(JsonElement element, string path)
    {
        var inlines = JsonContract.ExpectArray(element, path);
        var builder = new PreviewRunBuilder();
        var index = 0;
        foreach (var inline in inlines.EnumerateArray())
        {
            Append(inline, $"{path}/{index}", builder, false, false);
            index++;
        }
        return builder.Build();
    }

    private static void Append(
        JsonElement inline,
        string path,
        PreviewRunBuilder builder,
        bool strong,
        bool emphasis)
    {
        if (inline.ValueKind is not JsonValueKind.Object)
        {
            throw JsonContract.Error(path, "inline must be an object");
        }
        var type = JsonContract.RequiredString(inline, "type", path);
        switch (type)
        {
            case "text":
                JsonContract.ExpectObject(inline, path, ["type", "value"]);
                var value = JsonContract.RequiredString(inline, "value", path);
                if (value.Length == 0 || value.Any(c => c <= ' ' || c == '\u007f'))
                    throw JsonContract.Error(path + "/value", "text must be nonempty and contain no ASCII spaces or control characters; use space/line_break nodes");
                builder.Append(value, strong, emphasis);
                break;
            case "space":
                JsonContract.ExpectObject(inline, path, ["type"]);
                builder.Append(" ", strong, emphasis);
                break;
            case "line_break":
                JsonContract.ExpectObject(inline, path, ["type"]);
                builder.BreakLine();
                break;
            case "strong":
                JsonContract.ExpectObject(inline, path, ["type", "inlines"]);
                AppendChildren(
                    inline.GetProperty("inlines"),
                    path + "/inlines",
                    builder,
                    true,
                    emphasis);
                break;
            case "emph":
                JsonContract.ExpectObject(inline, path, ["type", "inlines"]);
                AppendChildren(
                    inline.GetProperty("inlines"),
                    path + "/inlines",
                    builder,
                    strong,
                    true);
                break;
            case "link":
                JsonContract.ExpectObject(inline, path, ["type", "target", "title", "inlines"]);
                _ = JsonContract.RequiredString(inline, "target", path);
                JsonContract.ExpectNullOrString(inline.GetProperty("title"), path + "/title");
                AppendChildren(
                    inline.GetProperty("inlines"),
                    path + "/inlines",
                    builder,
                    strong,
                    emphasis);
                break;
            default:
                throw JsonContract.Error(path + "/type", $"unsupported IR inline type {type}");
        }
    }

    private static void AppendChildren(
        JsonElement element,
        string path,
        PreviewRunBuilder builder,
        bool strong,
        bool emphasis)
    {
        var children = JsonContract.ExpectArray(element, path);
        var index = 0;
        foreach (var child in children.EnumerateArray())
        {
            Append(child, $"{path}/{index}", builder, strong, emphasis);
            index++;
        }
    }
}

internal sealed class PreviewRunBuilder
{
    private readonly List<List<PreviewTextRun>> lines = [[]];

    public void Append(string text, bool strong, bool emphasis)
    {
        if (text.Length == 0)
        {
            return;
        }
        var line = lines[^1];
        if (line.Count > 0 && line[^1].Strong == strong && line[^1].Emphasis == emphasis)
        {
            line[^1] = line[^1] with { Text = line[^1].Text + text };
            return;
        }
        line.Add(new PreviewTextRun(text, strong, emphasis));
    }

    public void BreakLine() => lines.Add([]);

    public PreviewInlineContent Build() => new(lines.Select(line => new PreviewLine(
        string.Concat(line.Select(run => run.Text)),
        line.ToArray())).ToArray());

    public static IReadOnlyList<PreviewTextRun> Coalesce(IEnumerable<PreviewTextRun> source)
    {
        var builder = new PreviewRunBuilder();
        foreach (var run in source)
        {
            builder.Append(run.Text, run.Strong, run.Emphasis);
        }
        return builder.lines[0].ToArray();
    }
}

internal static class JsonContract
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
        IReadOnlyCollection<string> required,
        IReadOnlyCollection<string>? optional = null)
    {
        if (element.ValueKind is not JsonValueKind.Object)
        {
            throw Error(path, "expected object");
        }
        var allowed = new HashSet<string>(required, StringComparer.Ordinal);
        if (optional is not null)
        {
            allowed.UnionWith(optional);
        }
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

    public static JsonElement ExpectArray(JsonElement element, string path)
    {
        if (element.ValueKind is not JsonValueKind.Array)
        {
            throw Error(path, "expected array");
        }
        return element;
    }

    public static string RequiredString(JsonElement parent, string name, string path) =>
        ReadString(parent.GetProperty(name), path + "/" + name);

    public static string ReadString(JsonElement element, string path) =>
        element.ValueKind is JsonValueKind.String
            ? element.GetString()!
            : throw Error(path, "expected string");

    public static int RequiredInt32(JsonElement parent, string name, string path) =>
        ReadInt32(parent.GetProperty(name), path + "/" + name);

    public static int ReadInt32(JsonElement element, string path) =>
        element.ValueKind is JsonValueKind.Number && element.TryGetInt32(out var value)
            ? value
            : throw Error(path, "expected 32-bit integer");

    public static bool RequiredBoolean(JsonElement parent, string name, string path)
    {
        var element = parent.GetProperty(name);
        return element.ValueKind switch
        {
            JsonValueKind.True => true,
            JsonValueKind.False => false,
            _ => throw Error(path + "/" + name, "expected boolean"),
        };
    }

    public static void ExpectString(JsonElement element, string path, string expected)
    {
        var actual = ReadString(element, path);
        if (!string.Equals(actual, expected, StringComparison.Ordinal))
        {
            throw Error(path, $"expected {expected}, got {actual}");
        }
    }

    public static void ExpectNullOrString(JsonElement element, string path)
    {
        if (element.ValueKind is not (JsonValueKind.Null or JsonValueKind.String))
        {
            throw Error(path, "expected null or string");
        }
    }

    public static InvalidDataException Error(string path, string message) =>
        new($"invalid preview IR at {(path.Length == 0 ? "/" : path)}: {message}");
}

internal static class PathSafety
{
    public static bool IsWithin(string root, string candidate)
    {
        var rootPath = Path.GetFullPath(root).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
        var candidatePath = Path.GetFullPath(candidate);
        return candidatePath.StartsWith(rootPath, StringComparison.OrdinalIgnoreCase);
    }
}

internal static class PngDimensions
{
    private static readonly byte[] Signature = [137, 80, 78, 71, 13, 10, 26, 10];

    public static (int Width, int Height) Read(string path)
    {
        Span<byte> header = stackalloc byte[24];
        using var stream = File.OpenRead(path);
        if (stream.Read(header) != header.Length || !header[..8].SequenceEqual(Signature))
        {
            throw new InvalidDataException($"Invalid PNG header: {path}");
        }
        var width = BinaryPrimitives.ReadInt32BigEndian(header[16..20]);
        var height = BinaryPrimitives.ReadInt32BigEndian(header[20..24]);
        if (width <= 0 || height <= 0)
        {
            throw new InvalidDataException($"Invalid PNG dimensions: {path}");
        }
        return (width, height);
    }
}
