using System.Diagnostics;
using System.Net;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Xml.Linq;
using Microsoft.Win32;

namespace Md2Hwp.HancomIrPreview;

internal sealed record ProbeResult(
    string Template,
    string ModulePath,
    bool RegisterModule,
    bool Open,
    bool TemplateUnchanged);

internal sealed record RenderResult(
    string Output,
    int TextOperations,
    int BoxOperations,
    int FigureOperations,
    int BoxesAdded,
    int CaptionsAdded,
    int PicturesAdded,
    IReadOnlyList<StyleBinding> StyleBindings,
    bool TextVerified,
    bool StylesVerified,
    bool CharacterMarksVerified,
    bool BoxesVerified,
    bool CaptionsVerified,
    bool TemplateUnchanged);

internal sealed record StyleBinding(
    string Symbolic,
    string NativeName,
    int NativeId);

internal sealed record PreviewRendering(
    int BoxesAdded,
    int CaptionsAdded,
    int PicturesAdded,
    IReadOnlyList<StyleBinding> StyleBindings);

internal sealed record SavedParagraph(
    int Style,
    string Text,
    bool ContainsPicture,
    bool ContainsTable,
    int FigureAutoNumbers,
    IReadOnlyList<SavedTextRun> Runs);

internal sealed record SavedTextRun(
    string Text,
    bool Bold,
    bool Italic);

internal sealed record ExpectedParagraph(
    string SymbolicStyle,
    string Text,
    bool ContainsPicture,
    bool ContainsTable,
    int FigureAutoNumbers,
    IReadOnlyList<PreviewTextRun>? FormattedRuns);

internal sealed record ExportedPage(
    string Path,
    int PixelWidth,
    int PixelHeight,
    string Sha256);

internal sealed record ImageExportResult(
    string Document,
    string OutputDirectory,
    IReadOnlyList<ExportedPage> Pages,
    bool DocumentUnchanged);

internal static class HancomPreviewWriter
{
    private const string ProgId = "HWPFrame.HwpObject";
    private const string ModuleName = "FilePathCheckerModuleExample";
    private const string OpenOptions = "lock:false;forceopen:true;suspendpassword:true;versionwarning:false";

    public static ProbeResult Probe(string templatePath, string repositoryRoot, bool visible)
    {
        var template = ValidateTemplate(templatePath);
        EnsureInteractiveContext();
        EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(repositoryRoot);
        var hashBefore = HashFile(template);

        WithHwp(module, hwp =>
        {
            Open(hwp, template, visible);
            return true;
        });

        var unchanged = string.Equals(hashBefore, HashFile(template), StringComparison.Ordinal);
        if (!unchanged)
        {
            throw new InvalidOperationException("The open-only probe changed its source template.");
        }
        return new ProbeResult(template, module.ModulePath, true, true, true);
    }

    public static RenderResult Render(
        IrPreviewPlan plan,
        string templatePath,
        string outputPath,
        string repositoryRoot,
        bool visible)
    {
        var template = ValidateTemplate(templatePath);
        var output = Path.GetFullPath(outputPath);
        if (!string.Equals(Path.GetExtension(output), ".hwp", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("The investigation preview currently preserves HWP only.");
        }
        if (string.Equals(template, output, StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("Output must not overwrite the source template.");
        }
        if (File.Exists(output))
        {
            throw new IOException($"Output already exists: {output}");
        }
        var outputDirectory = Path.GetDirectoryName(output)!;
        if (!Directory.Exists(outputDirectory))
        {
            throw new DirectoryNotFoundException($"Missing output directory: {outputDirectory}");
        }

        EnsureInteractiveContext();
        EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(repositoryRoot);
        var templateHashBefore = HashFile(template);
        var temporaryOutput = Path.Combine(
            outputDirectory,
            $".md2hwp-ir-preview-{Guid.NewGuid():N}.hwp");
        File.Copy(template, temporaryOutput, overwrite: false);

        try
        {
            var rendering = WithHwp(module, hwp =>
            {
                Open(hwp, temporaryOutput, visible);
                AuriPreviewStyleBindings styles = AuriPreviewStyleBindings.Bind(hwp);
                AuriMinimalBoxPrototype? boxPrototype = plan.Summary.BoxOperations == 0
                    ? null
                    : AuriMinimalBoxPrototype.Bind(hwp, styles);
                AuriMinimalCaptionPrototype? captionPrototype = plan.Summary.FigureOperations == 0
                    ? null
                    : AuriMinimalCaptionPrototype.Bind(hwp, styles);
                int picturesBefore = CountPictures(hwp);
                int captionsBefore = CountFigureAutoNumbers(hwp);
                IReadOnlyList<SavedParagraph> existingParagraphs = ReadParagraphs(hwp);
                var paragraphsBefore = existingParagraphs.Count;
                Run(hwp, "MoveDocEnd");
                Run(hwp, "BreakPara");

                foreach (var operation in plan.Operations)
                {
                    RenderOperation(
                        hwp,
                        operation,
                        styles,
                        boxPrototype,
                        captionPrototype);
                }

                Run(hwp, "FileSave");
                CloseDocument(hwp);
                Open(hwp, temporaryOutput, visible);

                VerifyText(hwp, plan);
                VerifyStyles(hwp, plan, styles, paragraphsBefore);
                VerifyCharacterMarks(hwp, plan, styles, paragraphsBefore);
                VerifyBoxes(hwp, plan, styles, boxPrototype, paragraphsBefore);
                VerifyCaptions(hwp, plan, styles, captionPrototype, paragraphsBefore);
                var picturesAfter = CountPictures(hwp);
                var picturesAdded = picturesAfter - picturesBefore;
                if (picturesAdded != plan.Summary.FigureOperations)
                {
                    throw new InvalidOperationException(
                        $"Expected {plan.Summary.FigureOperations} inserted pictures, " +
                        $"observed {picturesAdded}.");
                }
                var captionsAdded = CountFigureAutoNumbers(hwp) - captionsBefore;
                if (captionsAdded != plan.Summary.FigureOperations)
                {
                    throw new InvalidOperationException(
                        $"Expected {plan.Summary.FigureOperations} inserted figure captions, " +
                        $"observed {captionsAdded}.");
                }
                return new PreviewRendering(
                    plan.Summary.BoxOperations,
                    captionsAdded,
                    picturesAdded,
                    styles.Bindings);
            });

            var templateUnchanged = string.Equals(
                templateHashBefore,
                HashFile(template),
                StringComparison.Ordinal);
            if (!templateUnchanged)
            {
                throw new InvalidOperationException("The source template changed during preview rendering.");
            }

            File.Move(temporaryOutput, output);
            return new RenderResult(
                output,
                plan.Summary.TextOperations,
                plan.Summary.BoxOperations,
                plan.Summary.FigureOperations,
                rendering.BoxesAdded,
                rendering.CaptionsAdded,
                rendering.PicturesAdded,
                rendering.StyleBindings,
                true,
                true,
                true,
                true,
                true,
                true);
        }
        finally
        {
            if (File.Exists(temporaryOutput))
            {
                File.Delete(temporaryOutput);
            }
        }
    }

    public static ImageExportResult ExportImages(
        string documentPath,
        string outputDirectoryPath,
        string repositoryRoot,
        bool visible)
    {
        var document = ValidateTemplate(documentPath);
        var outputDirectory = Path.GetFullPath(outputDirectoryPath);
        if (File.Exists(outputDirectory) || Directory.Exists(outputDirectory))
        {
            throw new IOException($"Image output path already exists: {outputDirectory}");
        }
        var outputParent = Path.GetDirectoryName(outputDirectory)!;
        if (!Directory.Exists(outputParent))
        {
            throw new DirectoryNotFoundException($"Missing image output parent: {outputParent}");
        }

        EnsureInteractiveContext();
        EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(repositoryRoot);
        var documentHashBefore = HashFile(document);
        var temporaryDirectory = Path.Combine(
            outputParent,
            $".md2hwp-image-export-{Guid.NewGuid():N}");
        Directory.CreateDirectory(temporaryDirectory);

        try
        {
            var requestedImage = Path.Combine(temporaryDirectory, "page.png");
            WithHwp(module, hwp =>
            {
                Open(hwp, document, visible);
                object? saveResult = hwp.SaveAs(requestedImage, "PNG", string.Empty);
                if (!IndicatesSuccess(saveResult))
                {
                    var resultType = saveResult?.GetType().FullName ?? "null";
                    throw new InvalidOperationException(
                        $"Hancom returned an unsuccessful PNG SaveAs result ({resultType}).");
                }
                return true;
            });

            var imagePaths = Directory.GetFiles(temporaryDirectory, "*.png", SearchOption.TopDirectoryOnly)
                .OrderBy(path => path, StringComparer.OrdinalIgnoreCase)
                .ToArray();
            if (imagePaths.Length == 0)
            {
                throw new InvalidOperationException("Hancom PNG SaveAs produced no page images.");
            }

            var pages = imagePaths.Select(path =>
            {
                var (width, height) = PngDimensions.Read(path);
                return new ExportedPage(
                    Path.Combine(outputDirectory, Path.GetFileName(path)),
                    width,
                    height,
                    HashFile(path));
            }).ToArray();
            if (!string.Equals(documentHashBefore, HashFile(document), StringComparison.Ordinal))
            {
                throw new InvalidOperationException("The source document changed during PNG export.");
            }

            Directory.Move(temporaryDirectory, outputDirectory);
            return new ImageExportResult(document, outputDirectory, pages, true);
        }
        finally
        {
            if (Directory.Exists(temporaryDirectory))
            {
                Directory.Delete(temporaryDirectory, recursive: true);
            }
        }
    }

    private static T WithHwp<T>(SecurityModuleRegistration module, Func<dynamic, T> operation)
    {
        _ = module;
        object? comObject = null;
        dynamic? hwp = null;
        Exception? primaryFailure = null;
        Exception? cleanupFailure = null;
        T? result = default;

        try
        {
            var type = Type.GetTypeFromProgID(ProgId, throwOnError: true)!;
            comObject = Activator.CreateInstance(type)
                ?? throw new InvalidOperationException($"Could not create {ProgId}.");
            hwp = comObject;

            // This must remain the first COM call after object creation.
            if (!(bool)hwp.RegisterModule("FilePathCheckDLL", ModuleName))
            {
                throw new InvalidOperationException("Hancom rejected the registered file-access security module.");
            }

            result = operation(hwp);
        }
        catch (Exception error)
        {
            primaryFailure = error;
        }
        finally
        {
            if (hwp is not null)
            {
                try
                {
                    _ = hwp.Clear(1);
                }
                catch (Exception error)
                {
                    cleanupFailure = error;
                }
                try
                {
                    _ = hwp.Quit();
                }
                catch (Exception error)
                {
                    cleanupFailure = cleanupFailure is null
                        ? error
                        : new AggregateException(cleanupFailure, error);
                }
            }
            if (comObject is not null && Marshal.IsComObject(comObject))
            {
                _ = Marshal.FinalReleaseComObject(comObject);
            }
            GC.Collect();
            GC.WaitForPendingFinalizers();
        }

        Exception? exitFailure = null;
        try
        {
            WaitForHwpExit();
        }
        catch (Exception error)
        {
            exitFailure = error;
        }

        if (primaryFailure is not null)
        {
            throw new InvalidOperationException(
                "Hancom preview operation failed.",
                Combine(primaryFailure, cleanupFailure, exitFailure));
        }
        if (cleanupFailure is not null || exitFailure is not null)
        {
            throw new InvalidOperationException(
                "Hancom preview cleanup failed.",
                Combine(cleanupFailure, exitFailure));
        }
        return result!;
    }

    private static Exception Combine(params Exception?[] errors)
    {
        var present = errors.Where(error => error is not null).Cast<Exception>().ToArray();
        return present.Length == 1 ? present[0] : new AggregateException(present);
    }

    private static void Open(dynamic hwp, string path, bool visible)
    {
        if (!(bool)hwp.Open(path, "HWP", OpenOptions))
        {
            throw new InvalidOperationException($"Hancom could not open: {path}");
        }
        if (visible)
        {
            hwp.XHwpWindows.Item(0).Visible = true;
        }
    }

    private static void CloseDocument(dynamic hwp)
    {
        _ = hwp.Clear(1);
    }

    private static void RenderOperation(
        dynamic hwp,
        PreviewOperation operation,
        AuriPreviewStyleBindings styles,
        AuriMinimalBoxPrototype? boxPrototype,
        AuriMinimalCaptionPrototype? captionPrototype)
    {
        if (operation.Kind == "text")
        {
            var style = styles.Resolve(operation.ParagraphStyle ??
                throw new InvalidOperationException($"Missing paragraph style for {operation.Label}."));
            for (var index = 0; index < operation.Lines.Count; index++)
            {
                ApplyParagraphStyle(hwp, style);
                InsertFormattedLine(hwp, FormattedLine(operation, index), style);
                Run(hwp, "BreakPara");
            }
            return;
        }
        if (operation.Kind == "box")
        {
            if (boxPrototype is null ||
                !string.Equals(operation.ParagraphStyle, "block.box", StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"No minimal-fixture box prototype is bound for {operation.Label}.");
            }
            _ = boxPrototype.Insert(hwp, styles, operation.Lines);
            return;
        }
        if (operation.Kind != "figure" || operation.ImagePath is null ||
            operation.ImageWidthMillimeters is null || operation.ImageHeightMillimeters is null)
        {
            throw new InvalidOperationException($"Invalid preview operation: {operation.Kind}");
        }

        ApplyParagraphStyle(hwp, styles.Resolve("figure"));
        object? insertionResult = hwp.InsertPicture(
            operation.ImagePath,
            true,
            1,
            false,
            false,
            0,
            operation.ImageWidthMillimeters.Value,
            operation.ImageHeightMillimeters.Value);
        if (!IndicatesSuccess(insertionResult))
        {
            var resultType = insertionResult?.GetType().FullName ?? "null";
            throw new InvalidOperationException(
                $"Hancom returned an unexpected InsertPicture result ({resultType}): {operation.ImagePath}");
        }
        Run(hwp, "MoveParaEnd");
        Run(hwp, "BreakPara");
        if (captionPrototype is null)
        {
            throw new InvalidOperationException(
                $"No minimal-fixture figure-caption prototype is bound for {operation.Label}.");
        }
        _ = captionPrototype.Insert(
            hwp,
            styles,
            FormattedLine(operation, 1));
        var sourceStyle = styles.Resolve("figure.source");
        ApplyParagraphStyle(hwp, sourceStyle);
        var sourceRuns = new List<PreviewTextRun>
        {
            new(operation.Lines[2].Length == 0 ? "출처:" : "출처: ", false, false),
        };
        sourceRuns.AddRange(FormattedLine(operation, 2));
        InsertFormattedLine(hwp, PreviewRunBuilder.Coalesce(sourceRuns), sourceStyle);
        Run(hwp, "BreakPara");
    }

    private static IReadOnlyList<PreviewTextRun> FormattedLine(
        PreviewOperation operation,
        int index)
    {
        var formatted = operation.FormattedLines ??
            throw new InvalidOperationException($"Missing formatted lines for {operation.Label}.");
        if (formatted.Count != operation.Lines.Count)
        {
            throw new InvalidOperationException(
                $"Formatted-line count does not match plain lines for {operation.Label}.");
        }
        var runs = formatted[index];
        if (!string.Equals(
                string.Concat(runs.Select(run => run.Text)),
                operation.Lines[index],
                StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                $"Formatted runs do not match plain line {index} for {operation.Label}.");
        }
        return runs;
    }

    private static void ApplyParagraphStyle(dynamic hwp, NativeStyle style)
    {
        _ = hwp.HAction.GetDefault("StyleEx", hwp.HParameterSet.HStyle.HSet);
        hwp.HParameterSet.HStyle.Apply = style.Id;
        object? result = hwp.HAction.Execute("StyleEx", hwp.HParameterSet.HStyle.HSet);
        if (!IndicatesSuccess(result))
        {
            throw new InvalidOperationException(
                $"Hancom failed to apply paragraph style {style.Name} ({style.Id}).");
        }
    }

    private static void InsertText(dynamic hwp, string text)
    {
        _ = hwp.HAction.GetDefault("InsertText", hwp.HParameterSet.HInsertText.HSet);
        hwp.HParameterSet.HInsertText.Text = text;
        if (!(bool)hwp.HAction.Execute("InsertText", hwp.HParameterSet.HInsertText.HSet))
        {
            throw new InvalidOperationException("Hancom failed to insert preview text.");
        }
    }

    internal static void InsertFormattedLine(
        dynamic hwp,
        IReadOnlyList<PreviewTextRun> runs,
        NativeStyle baseStyle)
    {
        if (runs.Count == 0)
        {
            return;
        }

        var baseBold = baseStyle.BaseBold;
        var baseItalic = baseStyle.BaseItalic;
        var currentBold = baseBold;
        var currentItalic = baseItalic;
        Exception? primaryFailure = null;
        try
        {
            foreach (var run in runs)
            {
                SetCharacterMarks(
                    hwp,
                    baseBold || run.Strong,
                    baseItalic || run.Emphasis,
                    ref currentBold,
                    ref currentItalic);
                InsertText(hwp, run.Text);
            }
        }
        catch (Exception error)
        {
            primaryFailure = error;
        }

        Exception? resetFailure = null;
        try
        {
            SetCharacterMarks(
                hwp,
                baseBold,
                baseItalic,
                ref currentBold,
                ref currentItalic);
        }
        catch (Exception error)
        {
            resetFailure = error;
        }

        if (primaryFailure is not null)
        {
            throw new InvalidOperationException(
                "Hancom failed to insert formatted preview text.",
                Combine(primaryFailure, resetFailure));
        }
        if (resetFailure is not null)
        {
            throw new InvalidOperationException(
                "Hancom failed to restore the base character shape.",
                resetFailure);
        }
    }

    private static void SetCharacterMarks(
        dynamic hwp,
        bool bold,
        bool italic,
        ref bool currentBold,
        ref bool currentItalic)
    {
        if (currentBold != bold)
        {
            Run(hwp, "CharShapeBold");
            currentBold = bold;
        }
        if (currentItalic != italic)
        {
            Run(hwp, "CharShapeItalic");
            currentItalic = italic;
        }
    }

    private static void Run(dynamic hwp, string action)
    {
        if (!(bool)hwp.HAction.Run(action))
        {
            throw new InvalidOperationException($"Hancom action failed: {action}");
        }
    }

    private static bool IndicatesSuccess(object? result) => result switch
    {
        bool value => value,
        null => false,
        _ => Marshal.IsComObject(result),
    };

    private static void VerifyText(dynamic hwp, IrPreviewPlan plan)
    {
        var extracted = DecodeHwpTextTransport((string)hwp.GetTextFile("TEXT", ""));
        foreach (var expected in StyledTexts(plan).Select(item => item.Text).Where(text => text.Length > 0))
        {
            if (!extracted.Contains(expected, StringComparison.Ordinal))
            {
                throw new InvalidOperationException($"Saved preview did not contain expected text: {expected}");
            }
        }
    }

    private static void VerifyStyles(
        dynamic hwp,
        IrPreviewPlan plan,
        AuriPreviewStyleBindings styles,
        int paragraphsBefore)
    {
        IReadOnlyList<SavedParagraph> savedParagraphs = ReadParagraphs(hwp);
        var appended = savedParagraphs.Skip(paragraphsBefore).ToArray();
        var expectedParagraphs = ExpectedParagraphs(plan).ToArray();
        if (appended.Length < expectedParagraphs.Length)
        {
            throw new InvalidOperationException(
                $"Saved preview appended {appended.Length} paragraphs; expected at least {expectedParagraphs.Length}.");
        }

        for (var index = 0; index < expectedParagraphs.Length; index++)
        {
            var expected = expectedParagraphs[index];
            var actual = appended[index];
            var nativeStyle = styles.Resolve(expected.SymbolicStyle);
            if (actual.Style != nativeStyle.Id ||
                actual.ContainsPicture != expected.ContainsPicture ||
                actual.ContainsTable != expected.ContainsTable ||
                actual.FigureAutoNumbers != expected.FigureAutoNumbers ||
                !actual.Text.Contains(expected.Text, StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"Saved preview paragraph {index} did not match {expected.SymbolicStyle}/{nativeStyle.Name}: {expected.Text}");
            }
        }
    }

    private static void VerifyCharacterMarks(
        dynamic hwp,
        IrPreviewPlan plan,
        AuriPreviewStyleBindings styles,
        int paragraphsBefore)
    {
        IReadOnlyList<SavedParagraph> savedParagraphs = ReadParagraphs(hwp);
        var appended = savedParagraphs.Skip(paragraphsBefore).ToArray();
        var expectedParagraphs = ExpectedParagraphs(plan).ToArray();
        for (var index = 0; index < expectedParagraphs.Length; index++)
        {
            var expected = expectedParagraphs[index];
            if (expected.FormattedRuns is null)
            {
                continue;
            }
            var nativeStyle = styles.Resolve(expected.SymbolicStyle);
            var expectedRuns = CoalesceSavedRuns(expected.FormattedRuns.Select(run => new SavedTextRun(
                run.Text,
                nativeStyle.BaseBold || run.Strong,
                nativeStyle.BaseItalic || run.Emphasis)));
            var actualRuns = appended[index].Runs;
            if (actualRuns.Count != expectedRuns.Count ||
                actualRuns.Where((run, runIndex) => run != expectedRuns[runIndex]).Any())
            {
                throw new InvalidOperationException(
                    $"Saved preview character marks did not match paragraph {index} ({expected.SymbolicStyle}); " +
                    $"expected {DescribeRuns(expectedRuns)}, actual {DescribeRuns(actualRuns)}.");
            }
        }
    }

    private static string DescribeRuns(IEnumerable<SavedTextRun> runs) =>
        "[" + string.Join(
            ", ",
            runs.Select(run =>
                $"{JsonSerializer.Serialize(run.Text)}:bold={run.Bold}:italic={run.Italic}")) + "]";

    private static void VerifyBoxes(
        dynamic hwp,
        IrPreviewPlan plan,
        AuriPreviewStyleBindings styles,
        AuriMinimalBoxPrototype? prototype,
        int paragraphsBefore)
    {
        if (plan.Summary.BoxOperations == 0)
        {
            if (prototype is not null)
            {
                throw new InvalidOperationException("A box prototype was bound without a box operation.");
            }
            return;
        }
        if (prototype is null)
        {
            throw new InvalidOperationException("Missing minimal-fixture box prototype verification.");
        }

        XDocument document = AuriMinimalBoxPrototype.ReadDocument(hwp);
        IReadOnlyList<XElement> roots = AuriMinimalBoxPrototype.RootParagraphs(document);
        prototype.VerifyOriginal(roots);
        var appended = roots.Skip(paragraphsBefore).ToArray();
        var rootIndex = 0;
        var verified = 0;
        foreach (var operation in plan.Operations)
        {
            switch (operation.Kind)
            {
                case "text":
                    rootIndex += operation.Lines.Count;
                    break;
                case "box":
                    if (rootIndex >= appended.Length)
                    {
                        throw new InvalidOperationException("Saved preview lost an expected box root.");
                    }
                    prototype.VerifyRenderedRoot(
                        appended[rootIndex],
                        styles,
                        operation.Lines);
                    rootIndex++;
                    verified++;
                    break;
                case "figure":
                    rootIndex += 3;
                    break;
                default:
                    throw new InvalidOperationException(
                        $"Unknown preview operation during box verification: {operation.Kind}");
            }
        }
        if (verified != plan.Summary.BoxOperations)
        {
            throw new InvalidOperationException(
                $"Expected {plan.Summary.BoxOperations} verified boxes, observed {verified}.");
        }
    }

    private static void VerifyCaptions(
        dynamic hwp,
        IrPreviewPlan plan,
        AuriPreviewStyleBindings styles,
        AuriMinimalCaptionPrototype? prototype,
        int paragraphsBefore)
    {
        if (plan.Summary.FigureOperations == 0)
        {
            if (prototype is not null)
            {
                throw new InvalidOperationException(
                    "A caption prototype was bound without a figure operation.");
            }
            return;
        }
        if (prototype is null)
        {
            throw new InvalidOperationException(
                "Missing minimal-fixture figure-caption prototype verification.");
        }

        XDocument document = AuriMinimalCaptionPrototype.ReadDocument(hwp);
        IReadOnlyList<XElement> roots = AuriMinimalCaptionPrototype.RootParagraphs(document);
        prototype.VerifyOriginal(roots);
        var appended = roots.Skip(paragraphsBefore).ToArray();
        var rootIndex = 0;
        var verified = 0;
        foreach (var operation in plan.Operations)
        {
            switch (operation.Kind)
            {
                case "text":
                    rootIndex += operation.Lines.Count;
                    break;
                case "box":
                    rootIndex++;
                    break;
                case "figure":
                    if (rootIndex + 1 >= appended.Length)
                    {
                        throw new InvalidOperationException(
                            "Saved preview lost an expected automatic-number caption root.");
                    }
                    prototype.VerifyRenderedRoot(
                        appended[rootIndex + 1],
                        styles,
                        operation.Lines[1]);
                    rootIndex += 3;
                    verified++;
                    break;
                default:
                    throw new InvalidOperationException(
                        $"Unknown preview operation during caption verification: {operation.Kind}");
            }
        }
        if (verified != plan.Summary.FigureOperations)
        {
            throw new InvalidOperationException(
                $"Expected {plan.Summary.FigureOperations} verified captions, observed {verified}.");
        }
    }

    private static IEnumerable<(string Symbolic, string Text)> StyledTexts(IrPreviewPlan plan)
    {
        foreach (var operation in plan.Operations)
        {
            switch (operation.Kind)
            {
                case "text":
                    var symbolicStyle = operation.ParagraphStyle ??
                        throw new InvalidOperationException(
                            $"Missing paragraph style for {operation.Label}.");
                    foreach (var line in operation.Lines)
                    {
                        yield return (symbolicStyle, line);
                    }
                    break;
                case "box":
                    foreach (var line in operation.Lines)
                    {
                        yield return ("block.box", line);
                    }
                    break;
                case "figure":
                    yield return ("figure.caption", operation.Lines[1]);
                    yield return (
                        "figure.source",
                        operation.Lines[2].Length == 0
                            ? "출처:"
                            : $"출처: {operation.Lines[2]}");
                    break;
                default:
                    throw new InvalidOperationException(
                        $"Unknown preview operation during text verification: {operation.Kind}");
            }
        }
    }

    private static IEnumerable<ExpectedParagraph> ExpectedParagraphs(IrPreviewPlan plan)
    {
        foreach (var operation in plan.Operations)
        {
            switch (operation.Kind)
            {
                case "text":
                    var symbolicStyle = operation.ParagraphStyle ??
                        throw new InvalidOperationException(
                            $"Missing paragraph style for {operation.Label}.");
                    for (var index = 0; index < operation.Lines.Count; index++)
                    {
                        yield return new ExpectedParagraph(
                            symbolicStyle,
                            operation.Lines[index],
                            false,
                            false,
                            0,
                            FormattedLine(operation, index));
                    }
                    break;
                case "box":
                    yield return new ExpectedParagraph(
                        "body",
                        string.Concat(operation.Lines),
                        false,
                        true,
                        0,
                        null);
                    break;
                case "figure":
                    yield return new ExpectedParagraph(
                        "figure",
                        string.Empty,
                        true,
                        false,
                        0,
                        null);
                    var captionRuns = new List<PreviewTextRun>
                    {
                        new("[그림 ] ", false, false),
                    };
                    captionRuns.AddRange(FormattedLine(operation, 1));
                    yield return new ExpectedParagraph(
                        "figure.caption",
                        operation.Lines[1],
                        false,
                        false,
                        1,
                        PreviewRunBuilder.Coalesce(captionRuns));
                    var sourceRuns = new List<PreviewTextRun>
                    {
                        new(operation.Lines[2].Length == 0 ? "출처:" : "출처: ", false, false),
                    };
                    sourceRuns.AddRange(FormattedLine(operation, 2));
                    yield return new ExpectedParagraph(
                        "figure.source",
                        operation.Lines[2].Length == 0
                            ? "출처:"
                            : $"출처: {operation.Lines[2]}",
                        false,
                        false,
                        0,
                        PreviewRunBuilder.Coalesce(sourceRuns));
                    break;
                default:
                    throw new InvalidOperationException(
                        $"Unknown preview operation during paragraph verification: {operation.Kind}");
            }
        }
    }

    private static string DecodeHwpTextTransport(string transport)
    {
        var codeUnitsDecoded = Regex.Replace(
            transport,
            @"&#([0-9]+);",
            match =>
            {
                if (!int.TryParse(match.Groups[1].Value, out var value) ||
                    value is < 0 or > 0x10FFFF)
                {
                    return match.Value;
                }
                return value <= char.MaxValue
                    ? new string((char)value, 1)
                    : char.ConvertFromUtf32(value);
            });
        return WebUtility.HtmlDecode(codeUnitsDecoded);
    }

    private static int CountPictures(dynamic hwp)
    {
        var xml = (string)hwp.GetTextFile("HWPML2X", "");
        return XDocument.Parse(xml)
            .Descendants()
            .Count(element => element.Name.LocalName == "PICTURE");
    }

    private static int CountFigureAutoNumbers(dynamic hwp) =>
        AuriMinimalCaptionPrototype.CountFigureAutoNumbers(
            XDocument.Parse((string)hwp.GetTextFile("HWPML2X", "")));

    private static IReadOnlyList<SavedParagraph> ReadParagraphs(dynamic hwp)
    {
        var document = XDocument.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        var characterShapes = HwpmlCharacterShapes.Read(document);
        return document.Descendants()
            .Where(element => element.Name.LocalName == "SECTION")
            .SelectMany(section => section.Elements()
                .Where(element => element.Name.LocalName == "P"))
            .Select(element => new SavedParagraph(
                int.TryParse(element.Attribute("Style")?.Value, out var style) ? style : -1,
                element.Value,
                element.Descendants().Any(descendant => descendant.Name.LocalName == "PICTURE"),
                element.Descendants().Any(descendant => descendant.Name.LocalName == "TABLE"),
                element.Descendants().Count(descendant =>
                    descendant.Name.LocalName == "AUTONUM" &&
                    string.Equals(
                        descendant.Attribute("NumberType")?.Value,
                        "Figure",
                        StringComparison.Ordinal)),
                ReadSavedRuns(element, characterShapes)))
            .ToArray();
    }

    private static IReadOnlyList<SavedTextRun> ReadSavedRuns(
        XElement paragraph,
        IReadOnlyDictionary<int, CharacterMarks> characterShapes)
    {
        var runs = new List<SavedTextRun>();
        foreach (var textElement in paragraph.Elements()
                     .Where(element => element.Name.LocalName == "TEXT"))
        {
            if (!int.TryParse(textElement.Attribute("CharShape")?.Value, out var characterShapeId) ||
                !characterShapes.TryGetValue(characterShapeId, out var marks))
            {
                throw new InvalidOperationException(
                    "HWPML TEXT did not reference a known character shape.");
            }
            var text = string.Concat(textElement.Elements()
                .Where(element => element.Name.LocalName == "CHAR")
                .Select(element => element.Value));
            if (text.Length > 0)
            {
                runs.Add(new SavedTextRun(text, marks.Bold, marks.Italic));
            }
        }
        return CoalesceSavedRuns(runs);
    }

    private static IReadOnlyList<SavedTextRun> CoalesceSavedRuns(
        IEnumerable<SavedTextRun> source)
    {
        var result = new List<SavedTextRun>();
        foreach (var run in source.Where(run => run.Text.Length > 0))
        {
            if (result.Count > 0 &&
                result[^1].Bold == run.Bold &&
                result[^1].Italic == run.Italic)
            {
                result[^1] = result[^1] with { Text = result[^1].Text + run.Text };
            }
            else
            {
                result.Add(run);
            }
        }
        return result;
    }

    private static string ValidateTemplate(string path)
    {
        var fullPath = Path.GetFullPath(path);
        if (!File.Exists(fullPath))
        {
            throw new FileNotFoundException("Missing HWP template.", fullPath);
        }
        if (!string.Equals(Path.GetExtension(fullPath), ".hwp", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("The investigation preview currently supports HWP input only.");
        }
        return fullPath;
    }

    private static void EnsureInteractiveContext()
    {
        if (!OperatingSystem.IsWindows() || !Environment.Is64BitProcess || !Environment.UserInteractive ||
            Process.GetCurrentProcess().SessionId == 0 ||
            Thread.CurrentThread.GetApartmentState() is not ApartmentState.STA)
        {
            throw new InvalidOperationException(
                "Hancom preview requires an interactive Windows x64 STA process outside session 0.");
        }
    }

    private static void EnsureNoExistingHwpProcess()
    {
        var processes = Process.GetProcessesByName("Hwp");
        if (processes.Length > 0)
        {
            throw new InvalidOperationException(
                "Close existing HWP processes before the isolated preview: " +
                string.Join(", ", processes.Select(process => process.Id)));
        }
    }

    private static void WaitForHwpExit()
    {
        var deadline = DateTime.UtcNow.AddSeconds(15);
        while (DateTime.UtcNow < deadline && Process.GetProcessesByName("Hwp").Length > 0)
        {
            Thread.Sleep(200);
        }
        var remaining = Process.GetProcessesByName("Hwp");
        if (remaining.Length > 0)
        {
            throw new InvalidOperationException(
                "HWP remained after COM cleanup; no process was force-stopped: " +
                string.Join(", ", remaining.Select(process => process.Id)));
        }
    }

    private static string HashFile(string path) =>
        Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));

}

internal sealed record CharacterMarks(bool Bold, bool Italic);

internal static class HwpmlCharacterShapes
{
    public static IReadOnlyDictionary<int, CharacterMarks> Read(XDocument document)
    {
        var result = new Dictionary<int, CharacterMarks>();
        foreach (var element in document.Descendants()
                     .Where(element => element.Name.LocalName == "CHARSHAPE"))
        {
            if (!int.TryParse(element.Attribute("Id")?.Value, out var id) ||
                !result.TryAdd(
                    id,
                    new CharacterMarks(
                        element.Elements().Any(child => child.Name.LocalName == "BOLD"),
                        element.Elements().Any(child => child.Name.LocalName == "ITALIC"))))
            {
                throw new InvalidOperationException(
                    "HWPML character-shape definitions must have unique integer IDs.");
            }
        }
        if (result.Count == 0)
        {
            throw new InvalidOperationException("HWPML did not contain character-shape definitions.");
        }
        return result;
    }
}

internal sealed record NativeStyle(
    int Id,
    string Name,
    bool BaseBold,
    bool BaseItalic);

internal sealed class AuriPreviewStyleBindings
{
    private static readonly (string Symbolic, string NativeName)[] Required =
    [
        ("body", "본문"),
        ("heading.1", "장제목 (개요 1)"),
        ("heading.2", "1. (개요 2)"),
        ("heading.3", "1) (개요 3)"),
        ("heading.4", "① (개요 4)"),
        ("heading.5", "□ (개요 5)"),
        ("heading.6", "․ (개요 6)"),
        ("block.box", "박스내용"),
        ("figure", "본문"),
        ("figure.caption", "표그림_캡션"),
        ("figure.source", "출처 및 하단설명"),
    ];

    private readonly IReadOnlyDictionary<string, NativeStyle> styles;

    private AuriPreviewStyleBindings(IReadOnlyDictionary<string, NativeStyle> styles)
    {
        this.styles = styles;
        Bindings = Required
            .Select(required => new StyleBinding(
                required.Symbolic,
                required.NativeName,
                styles[required.Symbolic].Id))
            .ToArray();
    }

    public IReadOnlyList<StyleBinding> Bindings { get; }

    public static AuriPreviewStyleBindings Bind(dynamic hwp)
    {
        var document = XDocument.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        return BindDocument(document);
    }

    public static AuriPreviewStyleBindings BindDocument(XDocument document)
    {
        var styleElements = document.Descendants()
            .Where(element => element.Name.LocalName == "STYLE")
            .ToArray();
        var characterShapes = HwpmlCharacterShapes.Read(document);
        var bindings = new Dictionary<string, NativeStyle>(StringComparer.Ordinal);

        foreach (var required in Required)
        {
            var matches = styleElements.Where(element =>
                    string.Equals(element.Attribute("Type")?.Value, "Para", StringComparison.Ordinal) &&
                    string.Equals(element.Attribute("Name")?.Value, required.NativeName, StringComparison.Ordinal))
                .ToArray();
            if (matches.Length != 1 ||
                !int.TryParse(matches[0].Attribute("Id")?.Value, out var nativeId) ||
                !int.TryParse(matches[0].Attribute("CharShape")?.Value, out var characterShapeId) ||
                !characterShapes.TryGetValue(characterShapeId, out var characterMarks))
            {
                throw new InvalidOperationException(
                    $"Expected one complete AURI paragraph style named {required.NativeName}, found {matches.Length}.");
            }
            bindings.Add(required.Symbolic, new NativeStyle(
                nativeId,
                required.NativeName,
                characterMarks.Bold,
                characterMarks.Italic));
        }

        return new AuriPreviewStyleBindings(bindings);
    }

    public NativeStyle Resolve(string symbolic)
    {
        return styles.TryGetValue(symbolic, out var style)
            ? style
            : throw new InvalidOperationException($"No AURI preview style binding for {symbolic}.");
    }
}

internal sealed record SecurityModuleRegistration(string ModulePath, string Sha256)
{
    public static SecurityModuleRegistration ReadAndValidate(string repositoryRoot)
    {
        var lockPath = Path.Combine(repositoryRoot, "dependencies", "lock.json");
        using var lockDocument = JsonDocument.Parse(File.ReadAllBytes(lockPath));
        var pins = lockDocument.RootElement.GetProperty("dependencies")
            .EnumerateArray()
            .Where(dependency => dependency.GetProperty("name").GetString() == "hancom-automation")
            .SelectMany(dependency => dependency.GetProperty("pins").EnumerateArray())
            .Where(pin => pin.GetProperty("name").GetString() == "file-path-checker-module-example")
            .ToArray();
        if (pins.Length != 1)
        {
            throw new InvalidOperationException("Expected one locked Hancom security module.");
        }
        var expectedHash = pins[0].GetProperty("sha256").GetString()!;

        using var registryKey = Registry.CurrentUser.OpenSubKey(
            @"Software\HNC\HwpAutomation\Modules",
            writable: false);
        if (registryKey is null || !registryKey.GetValueNames().Contains(ModuleName, StringComparer.Ordinal))
        {
            throw new InvalidOperationException("The Hancom security-module registration is missing.");
        }
        if (registryKey.GetValueKind(ModuleName) is not RegistryValueKind.String)
        {
            throw new InvalidOperationException("The Hancom security-module registration must be REG_SZ.");
        }
        var registeredPath = registryKey.GetValue(
            ModuleName,
            null,
            RegistryValueOptions.DoNotExpandEnvironmentNames) as string;
        if (string.IsNullOrWhiteSpace(registeredPath) || !Path.IsPathRooted(registeredPath))
        {
            throw new InvalidOperationException("The Hancom security-module path must be absolute.");
        }
        var fullPath = Path.GetFullPath(registeredPath);
        if (!File.Exists(fullPath))
        {
            throw new FileNotFoundException("The registered Hancom security module is missing.", fullPath);
        }
        var actualHash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(fullPath)));
        if (!string.Equals(actualHash, expectedHash, StringComparison.Ordinal))
        {
            throw new InvalidOperationException("The registered Hancom security module hash does not match dependencies/lock.json.");
        }
        return new SecurityModuleRegistration(fullPath, actualHash);
    }

    private const string ModuleName = "FilePathCheckerModuleExample";
}
