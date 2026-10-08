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
        var version = JsonContract.RequiredString(document.RootElement, "ir_version", "");
        IrContract.RequireCurrent(version, "Input");

        var metadata = document.RootElement.GetProperty("metadata");
        var metadataValues = TemplateMetadata.Read(metadata);
        var blocks = JsonContract.ExpectArray(document.RootElement.GetProperty("blocks"), "/blocks");

        var builder = new PlanBuilder(
            Path.GetFullPath(irPath),
            Path.GetFullPath(repositoryRoot),
            profile, TemplateHeadingNumbers.Start(metadataValues));
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
    int ListItems,
    int TableOperations = 0);

internal sealed record PreviewOperation(
    string Kind,
    string Label,
    IReadOnlyList<string> Lines,
    string? ImagePath = null,
    double? ImageWidthMillimeters = null,
    double? ImageHeightMillimeters = null,
    string? ParagraphStyle = null,
    IReadOnlyList<IReadOnlyList<PreviewTextRun>>? FormattedLines = null,
    PreviewListMarker? ListMarker = null,
    IReadOnlyList<PreviewTextRun>? SourceRuns = null,
    int? Heading1Number = null,
    string? FigureId = null,
    string? HeadingId = null,
    PreviewTable? Table = null,
    PreviewListMarker? ListContinuation = null,
    IReadOnlyList<PreviewSourceParagraph>? Sources = null);

internal sealed record PreviewTable(
    IReadOnlyList<string> Columns,
    IReadOnlyList<PreviewInlineContent> Header,
    IReadOnlyList<IReadOnlyList<PreviewInlineContent>> Rows,
    PreviewInlineContent? Caption = null,
    PreviewInlineContent? Source = null);

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
    bool Emphasis,
    PreviewFootnote? Footnote = null,
    PreviewCrossReference? CrossReference = null);

internal sealed record PreviewCrossReference(string Kind, string Target);

internal sealed record PreviewFootnote(
    IReadOnlyList<PreviewInlineContent> Paragraphs);

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
    InvestigationTemplateProfile profile, int heading1Start = 1)
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
            case "table":
                AddTable(block, path);
                break;
            default:
                throw JsonContract.Error(path + "/type", $"unsupported IR block type {type}");
        }
    }

    public IrPreviewPlan Build(int sourceBlocks)
    {
        var numberedOperations = TemplateHeadingNumbers.Track(operations, heading1Start);
        FigureReferenceContract.Validate(numberedOperations);
        var textOperations = operations.Count(operation => operation.Kind == "text");
        var boxOperations = operations.Count(operation => operation.Kind == "code");
        var figureOperations = operations.Count(operation => operation.Kind == "figure");
        return new IrPreviewPlan(
            irPath,
            profile.Id,
            new PreviewSummary(
                sourceBlocks,
                textOperations,
                boxOperations,
                figureOperations,
                listItems,
                operations.Count(operation => operation.Kind == "table")),
            numberedOperations,
            [
                "This is an investigation preview, not backend lowering.",
                "AURI paragraph styles are bound by unique native names during render.",
                "Strong/emphasis marks are retained as character-shape runs; link targets remain flattened.",
                "verbatim_block maps to one prototype-backed code operation; render accepts only the uniquely matched minimal-fixture box structure.",
                "IR 0.4 box sources replace the template source slot; absent sources remove the native caption.",
                "Figure captions remain one prototype-backed native AUTONUM operation; render accepts only the uniquely matched minimal-fixture root-caption structure.",
                profile.PreserveParagraphLineBreaks
                    ? "IR line_break nodes remain native line breaks in the same paragraph."
                    : "IR line_break nodes outside verbatim blocks are previewed as separate HWP paragraphs.",
                $"Only trusted resource-root PNG/JPG/JPEG figures are inserted; width is limited to {profile.Figure.MaxWidthMillimeters} mm by the investigation profile and aspect ratio is preserved.",
            ]);
    }

    private void AddHeading(JsonElement block, string path)
    {
        JsonContract.ExpectObject(block, path, ["type", "level", "inlines"], ["id"]);
        var headingId = FigureReferenceContract.ReadOptionalId(block, path);
        var level = JsonContract.RequiredInt32(block, "level", path);
        if (level is < 1 or > 6)
        {
            throw JsonContract.Error(path + "/level", "heading level must be between 1 and 6");
        }
        var style = $"heading{level}";
        operations.Add(Text(style, style, InlineText.Read(block.GetProperty("inlines"), path + "/inlines")) with { HeadingId = headingId });
    }

    private void AddParagraph(JsonElement block, string path, string? listLabel)
    {
        AddParagraph(block, path, listLabel, null);
    }

    private void AddParagraph(
        JsonElement block,
        string path,
        string? listLabel,
        PreviewListMarker? listMarker,
        PreviewListMarker? listContinuation = null)
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
        operations.Add(Text(label, style, content, listMarker) with { ListContinuation = listContinuation });
    }

    private void AddVerbatimBlock(JsonElement block, string path)
    {
        JsonContract.ExpectObject(block, path, ["type", "lines"], ["source"]);
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
        var sources = PreviewSourceParagraph.ReadOptional(block, path);
        IReadOnlyList<PreviewTextRun>? sourceRuns = sources?.First().Content.Flatten(" / ").Runs;
        operations.Add(new PreviewOperation(
            "code",
            "verbatim_block",
            content.Lines.Select(line => line.Text).ToArray(),
            ParagraphStyle: "code",
            FormattedLines: content.Lines.Select(line => line.Runs).ToArray(),
            SourceRuns: sourceRuns, Sources: sources));
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
            var itemMarker = new PreviewListMarker(listId, kind, depth, listStart, itemNumber, itemIndex == 0);

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
                            markerPending ? itemMarker : null,
                            markerPending ? null : itemMarker);
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
        JsonContract.ExpectObject(block, path, ["type", "image", "caption", "source"], ["id"]);
        var figureId = FigureReferenceContract.ReadOptionalId(block, path);
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
            throw JsonContract.Error(path + "/image/path", "image must resolve to an existing file inside the resource root (working directory for tagged rendering)");
        }
        int pixelWidth, pixelHeight;
        try { (pixelWidth, pixelHeight) = FigureDimensions.Read(imagePath); }
        catch (InvalidDataException error) { throw JsonContract.Error(path + "/image/path", error.Message); }
        var width = profile.Figure.MaxWidthMillimeters;
        var height = width * pixelHeight / pixelWidth;
        var alt = InlineText.Read(image.GetProperty("alt"), path + "/image/alt", allowFootnotes: false, allowCrossReferences: false);
        var caption = InlineText.Read(block.GetProperty("caption"), path + "/caption", allowFootnotes: false, allowCrossReferences: false);
        var sources = PreviewSourceParagraph.ReadOptional(block, path);
        var source = sources?.First().Content ?? PreviewInlineContent.Plain([string.Empty]);
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
            figureLines.Select(line => line.Runs).ToArray(), FigureId: figureId, Sources: sources));
    }

    private void AddTable(JsonElement block, string path)
    {
        JsonContract.ExpectObject(block, path, ["type", "columns", "header", "rows"], ["caption", "source"]);
        var columnsElement = JsonContract.ExpectArray(block.GetProperty("columns"), path + "/columns");
        var columns = columnsElement.EnumerateArray().Select((value, index) =>
            JsonContract.ReadString(value, $"{path}/columns/{index}")).ToArray();
        if (columns.Length == 0)
            throw JsonContract.Error(path + "/columns", "table must have at least one column");
        if (columns.Any(alignment => alignment is not ("default" or "left" or "center" or "right")))
            throw JsonContract.Error(path + "/columns", "table column alignment must be default, left, center or right");

        IReadOnlyList<PreviewInlineContent> ReadRow(JsonElement element, string rowPath)
        {
            var cells = JsonContract.ExpectArray(element, rowPath);
            if (cells.GetArrayLength() != columns.Length)
                throw JsonContract.Error(rowPath, "table row must contain exactly one cell for every column");
            return cells.EnumerateArray().Select((cell, index) =>
                InlineText.Read(cell, $"{rowPath}/{index}")).ToArray();
        }

        PreviewInlineContent? ReadOptional(string name)
        {
            if (!block.TryGetProperty(name, out var value) || value.ValueKind == JsonValueKind.Null) return null;
            if (JsonContract.ExpectArray(value, path + "/" + name).GetArrayLength() == 0)
                throw JsonContract.Error(path + "/" + name, "table caption/source must not be empty when supplied");
            return InlineText.Read(value, path + "/" + name, allowFootnotes: false, allowCrossReferences: false);
        }

        var header = ReadRow(block.GetProperty("header"), path + "/header");
        var rowsElement = JsonContract.ExpectArray(block.GetProperty("rows"), path + "/rows");
        var rows = rowsElement.EnumerateArray().Select((row, index) =>
            ReadRow(row, $"{path}/rows/{index}")).ToArray();
        var sources = PreviewSourceParagraph.ReadOptional(block, path);
        var table = new PreviewTable(columns, header, rows, ReadOptional("caption"), sources?.First().Content);
        var marker = "MD2HWP_GENERATED_TABLE_" + Guid.NewGuid().ToString("N");
        operations.Add(new PreviewOperation("table", "table", [marker], ParagraphStyle: "body",
            FormattedLines: PreviewInlineContent.Plain([marker]).Lines.Select(line => line.Runs).ToArray(), Table: table, Sources: sources));
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
    public static PreviewInlineContent Read(JsonElement element, string path, bool allowFootnotes = true, bool allowCrossReferences = true)
    {
        var inlines = JsonContract.ExpectArray(element, path);
        var builder = new PreviewRunBuilder();
        var index = 0;
        foreach (var inline in inlines.EnumerateArray())
        {
            Append(inline, $"{path}/{index}", builder, false, false, allowFootnotes, allowCrossReferences);
            index++;
        }
        return builder.Build();
    }

    private static void Append(
        JsonElement inline,
        string path,
        PreviewRunBuilder builder,
        bool strong,
        bool emphasis,
        bool allowFootnotes,
        bool allowCrossReferences)
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
                    emphasis,
                    allowFootnotes,
                    allowCrossReferences);
                break;
            case "emph":
                JsonContract.ExpectObject(inline, path, ["type", "inlines"]);
                AppendChildren(
                    inline.GetProperty("inlines"),
                    path + "/inlines",
                    builder,
                    strong,
                    true,
                    allowFootnotes,
                    allowCrossReferences);
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
                    emphasis,
                    allowFootnotes,
                    allowCrossReferences);
                break;
            case "cross_reference":
                if (!allowCrossReferences)
                    throw JsonContract.Error(path + "/type", "cross references are not supported in figure alt/caption or object sources");
                JsonContract.ExpectObject(inline, path, ["type", "kind", "target"]);
                var kind = JsonContract.RequiredString(inline, "kind", path);
                if (kind is not ("figure_number" or "heading_number"))
                    throw JsonContract.Error(path + "/kind", "supported reference kinds are figure_number and heading_number");
                var target = JsonContract.RequiredString(inline, "target", path);
                FigureReferenceContract.RequireId(target, path + "/target");
                builder.AppendCrossReference(new PreviewCrossReference(kind, target), strong, emphasis);
                break;
            case "footnote":
                if (!allowFootnotes)
                {
                    throw JsonContract.Error(path + "/type", "footnotes are not supported in this inline context, including nested footnotes");
                }
                JsonContract.ExpectObject(inline, path, ["type", "blocks"]);
                var blocks = JsonContract.ExpectArray(inline.GetProperty("blocks"), path + "/blocks");
                if (blocks.GetArrayLength() == 0)
                {
                    throw JsonContract.Error(path + "/blocks", "footnote must contain at least one paragraph");
                }
                var paragraphs = new List<PreviewInlineContent>();
                var blockIndex = 0;
                foreach (var block in blocks.EnumerateArray())
                {
                    var blockPath = $"{path}/blocks/{blockIndex}";
                    JsonContract.ExpectObject(block, blockPath, ["type", "inlines"]);
                    JsonContract.ExpectString(block.GetProperty("type"), blockPath + "/type", "paragraph");
                    var inlines = JsonContract.ExpectArray(block.GetProperty("inlines"), blockPath + "/inlines");
                    if (inlines.GetArrayLength() == 0)
                    {
                        throw JsonContract.Error(blockPath + "/inlines", "footnote paragraph must not be empty");
                    }
                    paragraphs.Add(Read(inlines, blockPath + "/inlines", allowFootnotes: false, allowCrossReferences: allowCrossReferences));
                    blockIndex++;
                }
                builder.AppendFootnote(new PreviewFootnote(paragraphs), strong, emphasis);
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
        bool emphasis,
        bool allowFootnotes,
        bool allowCrossReferences)
    {
        var children = JsonContract.ExpectArray(element, path);
        var index = 0;
        foreach (var child in children.EnumerateArray())
        {
            Append(child, $"{path}/{index}", builder, strong, emphasis, allowFootnotes, allowCrossReferences);
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
        if (line.Count > 0 && line[^1].Footnote is null && line[^1].CrossReference is null && line[^1].Strong == strong && line[^1].Emphasis == emphasis)
        {
            line[^1] = line[^1] with { Text = line[^1].Text + text };
            return;
        }
        line.Add(new PreviewTextRun(text, strong, emphasis));
    }

    public void AppendFootnote(PreviewFootnote footnote, bool strong, bool emphasis) =>
        lines[^1].Add(new PreviewTextRun(
            "MD2HWP_FOOTNOTE_" + Guid.NewGuid().ToString("N"), strong, emphasis, footnote));

    public void AppendCrossReference(PreviewCrossReference reference, bool strong, bool emphasis) =>
        lines[^1].Add(new PreviewTextRun("MD2HWP_CROSS_REFERENCE_" + Guid.NewGuid().ToString("N"),
            strong, emphasis, CrossReference: reference));

    public void BreakLine() => lines.Add([]);

    public PreviewInlineContent Build() => new(lines.Select(line => new PreviewLine(
        string.Concat(line.Select(run => run.Text)),
        line.ToArray())).ToArray());

    public static IReadOnlyList<PreviewTextRun> Coalesce(IEnumerable<PreviewTextRun> source)
    {
        var builder = new PreviewRunBuilder();
        foreach (var run in source)
        {
            if (run.Footnote is not null || run.CrossReference is not null)
            {
                builder.lines[^1].Add(run);
            }
            else
            {
                builder.Append(run.Text, run.Strong, run.Emphasis);
            }
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
