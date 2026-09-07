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
    string Profile,
    int TextOperations,
    int BoxOperations,
    int FigureOperations,
    int BoxesAdded,
    int CaptionsAdded,
    int PicturesAdded,
    int NativeListParagraphs,
    IReadOnlyList<StyleBinding> StyleBindings,
    bool TextVerified,
    bool StylesVerified,
    bool CharacterMarksVerified,
    bool BoxesVerified,
    bool CaptionsVerified,
    bool ListsVerified,
    bool TemplateUnchanged);

internal sealed record StyleBinding(
    string Symbolic,
    string NativeName,
    int NativeId);

internal sealed record PreviewRendering(
    int BoxesAdded,
    int CaptionsAdded,
    int PicturesAdded,
    int NativeListParagraphs,
    IReadOnlyList<StyleBinding> StyleBindings);

internal sealed record SavedParagraph(
    int Style,
    string Text,
    bool ContainsPicture,
    bool ContainsTable,
    int FigureAutoNumbers,
    SavedNativeList? NativeList,
    IReadOnlyList<SavedTextRun> Runs);

internal sealed record SavedNativeList(
    string Kind,
    int Level,
    int DefinitionId,
    int LeftMargin);

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
    PreviewListMarker? ListMarker,
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
        InvestigationTemplateProfile profile,
        string templatePath,
        string outputPath,
        string repositoryRoot,
        bool visible)
    {
        var template = ValidateTemplate(templatePath);
        if (!string.Equals(plan.ProfileId, profile.Id, StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                $"Preview plan profile {plan.ProfileId} does not match render profile {profile.Id}.");
        }
        profile.ValidateTemplate(template);
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
                AuriPreviewStyleBindings styles = AuriPreviewStyleBindings.Bind(hwp, profile);
                AuriMinimalBoxPrototype? boxPrototype = plan.Summary.BoxOperations == 0
                    ? null
                    : AuriMinimalBoxPrototype.Bind(hwp, styles);
                AuriMinimalCaptionPrototype? captionPrototype = plan.Summary.FigureOperations == 0
                    ? null
                    : AuriMinimalCaptionPrototype.Bind(hwp, styles);
                int picturesBefore = CountPictures(hwp);
                int captionsBefore = CountFigureAutoNumbers(hwp);
                int nativeListsBefore = CountNativeListParagraphs(hwp);
                IReadOnlyList<SavedParagraph> existingParagraphs = ReadParagraphs(hwp);
                var paragraphsBefore = existingParagraphs.Count;
                Run(hwp, "MoveDocEnd");
                Run(hwp, "BreakPara");

                int? activeListId = null;
                foreach (var operation in plan.Operations)
                {
                    activeListId = RenderOperation(
                        hwp,
                        operation,
                        styles,
                        boxPrototype,
                        captionPrototype,
                        activeListId);
                }
                if (activeListId is not null)
                {
                    ClearNativeListAtCaret(hwp, styles.Resolve("body"));
                }

                Run(hwp, "FileSave");
                CloseDocument(hwp);
                Open(hwp, temporaryOutput, visible);

                VerifyText(hwp, plan, profile);
                VerifyStyles(hwp, plan, styles, paragraphsBefore);
                VerifyCharacterMarks(hwp, plan, styles, paragraphsBefore);
                VerifyBoxes(hwp, plan, styles, boxPrototype, paragraphsBefore);
                VerifyCaptions(hwp, plan, styles, captionPrototype, paragraphsBefore);
                VerifyLists(hwp, plan, profile, paragraphsBefore);
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
                var nativeListParagraphs = CountNativeListParagraphs(hwp) - nativeListsBefore;
                if (nativeListParagraphs != plan.Summary.ListItems)
                {
                    throw new InvalidOperationException(
                        $"Expected {plan.Summary.ListItems} native list paragraphs, " +
                        $"observed {nativeListParagraphs}.");
                }
                return new PreviewRendering(
                    plan.Summary.BoxOperations,
                    captionsAdded,
                    picturesAdded,
                    nativeListParagraphs,
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
                profile.Id,
                plan.Summary.TextOperations,
                plan.Summary.BoxOperations,
                plan.Summary.FigureOperations,
                rendering.BoxesAdded,
                rendering.CaptionsAdded,
                rendering.PicturesAdded,
                rendering.NativeListParagraphs,
                rendering.StyleBindings,
                true,
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

    private static int? RenderOperation(
        dynamic hwp,
        PreviewOperation operation,
        AuriPreviewStyleBindings styles,
        AuriMinimalBoxPrototype? boxPrototype,
        AuriMinimalCaptionPrototype? captionPrototype,
        int? activeListId)
    {
        if (operation.Kind == "text")
        {
            var style = styles.Resolve(operation.ParagraphStyle ??
                throw new InvalidOperationException($"Missing paragraph style for {operation.Label}."));
            var marker = operation.ListMarker;
            for (var index = 0; index < operation.Lines.Count; index++)
            {
                if (index == 0 && marker is not null)
                {
                    if (activeListId != marker.ListId)
                    {
                        if (activeListId is null)
                        {
                            ApplyParagraphStyle(hwp, style);
                        }
                        ApplyNativeListMarker(hwp, marker, style, styles.Profile.Lists);
                    }
                }
                else
                {
                    if (activeListId is not null)
                    {
                        ApplyParagraphStyleAfterNativeList(hwp, styles, style);
                    }
                    else
                    {
                        ApplyParagraphStyle(hwp, style);
                    }
                }
                InsertFormattedLine(hwp, FormattedLine(operation, index), style);
                Run(hwp, "BreakPara");
            }
            return marker?.ListId;
        }
        if (operation.Kind == "box")
        {
            if (activeListId is not null)
            {
                ClearNativeListAtCaret(hwp, styles.Resolve("body"));
            }
            if (boxPrototype is null ||
                !string.Equals(operation.ParagraphStyle, "block.box", StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"No minimal-fixture box prototype is bound for {operation.Label}.");
            }
            _ = boxPrototype.Insert(hwp, styles, operation.Lines);
            return null;
        }
        if (operation.Kind != "figure" || operation.ImagePath is null ||
            operation.ImageWidthMillimeters is null || operation.ImageHeightMillimeters is null)
        {
            throw new InvalidOperationException($"Invalid preview operation: {operation.Kind}");
        }

        var figureStyle = styles.Resolve("figure");
        if (activeListId is not null)
        {
            ApplyParagraphStyleAfterNativeList(hwp, styles, figureStyle);
        }
        else
        {
            ApplyParagraphStyle(hwp, figureStyle);
        }
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
        var sourceLabel = styles.Profile.Figure.SourceLabel;
        var sourceRuns = new List<PreviewTextRun>
        {
            new(operation.Lines[2].Length == 0 ? sourceLabel : sourceLabel + " ", false, false),
        };
        sourceRuns.AddRange(FormattedLine(operation, 2));
        InsertFormattedLine(hwp, PreviewRunBuilder.Coalesce(sourceRuns), sourceStyle);
        Run(hwp, "BreakPara");
        return null;
    }

    private static void ClearNativeListAtCaret(dynamic hwp, NativeStyle bodyStyle)
    {
        _ = hwp.HAction.GetDefault(
            "ParagraphShape",
            hwp.HParameterSet.HParaShape.HSet);
        hwp.HParameterSet.HParaShape.HeadingType = 0;
        hwp.HParameterSet.HParaShape.Level = 0;
        hwp.HParameterSet.HParaShape.LeftMargin = bodyStyle.BaseLeftMargin;
        if (!(bool)hwp.HAction.Execute(
                "ParagraphShape",
                hwp.HParameterSet.HParaShape.HSet))
        {
            throw new InvalidOperationException(
                "Hancom failed to clear native list state at the current paragraph.");
        }
    }

    private static void ApplyParagraphStyleAfterNativeList(
        dynamic hwp,
        AuriPreviewStyleBindings styles,
        NativeStyle targetStyle)
    {
        var bodyStyle = styles.Resolve("body");
        ClearNativeListAtCaret(hwp, bodyStyle);
        if (targetStyle.Id == bodyStyle.Id)
        {
            ApplyParagraphStyle(hwp, styles.ResetStyle);
        }
        ApplyParagraphStyle(hwp, targetStyle);
    }

    private static void ApplyNativeListMarker(
        dynamic hwp,
        PreviewListMarker marker,
        NativeStyle bodyStyle,
        ProfileListLayout listLayout)
    {
        if (marker.Depth < 0 || marker.Depth > listLayout.MaxDepth)
        {
            throw new InvalidOperationException(
                $"Native list depth {marker.Depth} exceeds profile maximum {listLayout.MaxDepth}.");
        }
        if (string.Equals(marker.Kind, "bullet", StringComparison.Ordinal))
        {
            Run(hwp, "PutBullet");
            _ = hwp.HAction.GetDefault(
                "ParagraphShape",
                hwp.HParameterSet.HParaShape.HSet);
            hwp.HParameterSet.HParaShape.HeadingType = 3;
            hwp.HParameterSet.HParaShape.Level = marker.Depth;
            hwp.HParameterSet.HParaShape.LeftMargin =
                bodyStyle.BaseLeftMargin + (marker.Depth * listLayout.DepthIndentHwpUnits);
            if (!(bool)hwp.HAction.Execute(
                    "ParagraphShape",
                    hwp.HParameterSet.HParaShape.HSet))
            {
                throw new InvalidOperationException(
                    $"Hancom failed to apply a native bullet at depth {marker.Depth}.");
            }
            return;
        }
        if (!string.Equals(marker.Kind, "ordered", StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"Unsupported native list kind: {marker.Kind}");
        }
        Run(hwp, "PutParaNumber");
        _ = hwp.HAction.GetDefault(
            "ParagraphShape",
            hwp.HParameterSet.HParaShape.HSet);
        hwp.HParameterSet.HParaShape.HeadingType = 2;
        hwp.HParameterSet.HParaShape.Level = marker.Depth;
        hwp.HParameterSet.HParaShape.LeftMargin =
            bodyStyle.BaseLeftMargin + (marker.Depth * listLayout.DepthIndentHwpUnits);
        hwp.HParameterSet.HParaShape.Numbering.NewList = 1;
        hwp.HParameterSet.HParaShape.Numbering.StartNumber = marker.Number;
        SetNativeListLevelStart(
            hwp.HParameterSet.HParaShape.Numbering,
            marker.Depth,
            marker.Number);
        SetNativeListLevelNumberFormat(
            hwp.HParameterSet.HParaShape.Numbering,
            marker.Depth);
        if (!(bool)hwp.HAction.Execute(
                "ParagraphShape",
                hwp.HParameterSet.HParaShape.HSet))
        {
            throw new InvalidOperationException(
            $"Hancom failed to start a native ordered list at {marker.Number}.");
        }
    }

    private static void SetNativeListLevelStart(
        dynamic numbering,
        int depth,
        int number)
    {
        switch (depth)
        {
            case 0:
                numbering.StartNumber0 = number;
                break;
            case 1:
                numbering.StartNumber1 = number;
                break;
            case 2:
                numbering.StartNumber2 = number;
                break;
            case 3:
                numbering.StartNumber3 = number;
                break;
            case 4:
                numbering.StartNumber4 = number;
                break;
            case 5:
                numbering.StartNumber5 = number;
                break;
            case 6:
                numbering.StartNumber6 = number;
                break;
            default:
                throw new InvalidOperationException(
                    $"Native list depth is outside Hancom's supported range: {depth}");
        }
    }

    private static void SetNativeListLevelNumberFormat(
        dynamic numbering,
        int depth)
    {
        switch (depth)
        {
            case 0:
                numbering.NumFormatLevel0 = 0;
                break;
            case 1:
                numbering.NumFormatLevel1 = 0;
                break;
            case 2:
                numbering.NumFormatLevel2 = 0;
                break;
            case 3:
                numbering.NumFormatLevel3 = 0;
                break;
            case 4:
                numbering.NumFormatLevel4 = 0;
                break;
            case 5:
                numbering.NumFormatLevel5 = 0;
                break;
            case 6:
                numbering.NumFormatLevel6 = 0;
                break;
            default:
                throw new InvalidOperationException(
                    $"Native list depth is outside Hancom's supported range: {depth}");
        }
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

    private static void VerifyText(
        dynamic hwp,
        IrPreviewPlan plan,
        InvestigationTemplateProfile profile)
    {
        var extracted = DecodeHwpTextTransport((string)hwp.GetTextFile("TEXT", ""));
        foreach (var expected in StyledTexts(plan, profile).Select(item => item.Text).Where(text => text.Length > 0))
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
        var expectedParagraphs = ExpectedParagraphs(plan, styles.Profile).ToArray();
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
                !NativeListMatches(
                    actual.NativeList,
                    expected.ListMarker,
                    nativeStyle,
                    styles.Profile.Lists) ||
                !actual.Text.Contains(expected.Text, StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"Saved preview paragraph {index} did not match {expected.SymbolicStyle}/{nativeStyle.Name}: {expected.Text}");
            }
        }
    }

    private static bool NativeListMatches(
        SavedNativeList? actual,
        PreviewListMarker? expected,
        NativeStyle nativeStyle,
        ProfileListLayout listLayout) =>
        (actual, expected) switch
        {
            (null, null) => true,
            (not null, not null) =>
                string.Equals(actual.Kind, expected.Kind, StringComparison.Ordinal) &&
                actual.Level == expected.Depth &&
                actual.LeftMargin == nativeStyle.BaseLeftMargin +
                    (expected.Depth * listLayout.DepthIndentHwpUnits),
            _ => false,
        };

    private static void VerifyCharacterMarks(
        dynamic hwp,
        IrPreviewPlan plan,
        AuriPreviewStyleBindings styles,
        int paragraphsBefore)
    {
        IReadOnlyList<SavedParagraph> savedParagraphs = ReadParagraphs(hwp);
        var appended = savedParagraphs.Skip(paragraphsBefore).ToArray();
        var expectedParagraphs = ExpectedParagraphs(plan, styles.Profile).ToArray();
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

    private static void VerifyLists(
        dynamic hwp,
        IrPreviewPlan plan,
        InvestigationTemplateProfile profile,
        int paragraphsBefore)
    {
        var expected = ExpectedParagraphs(plan, profile).ToArray();
        IReadOnlyList<SavedParagraph> savedParagraphs = ReadParagraphs(hwp);
        var appended = savedParagraphs.Skip(paragraphsBefore).ToArray();
        if (appended.Length < expected.Length)
        {
            throw new InvalidOperationException(
                "Saved preview lost paragraphs before native-list verification.");
        }

        var document = XDocument.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        int? activeListId = null;
        int? activeDefinitionId = null;
        var verified = 0;
        for (var index = 0; index < expected.Length; index++)
        {
            var marker = expected[index].ListMarker;
            if (marker is null)
            {
                activeListId = null;
                activeDefinitionId = null;
                continue;
            }
            var actual = appended[index].NativeList ??
                throw new InvalidOperationException(
                    $"Saved preview paragraph {index} lost its native {marker.Kind} marker.");
            if (activeListId == marker.ListId)
            {
                if (actual.DefinitionId != activeDefinitionId)
                {
                    throw new InvalidOperationException(
                        $"Native list {marker.ListId} changed definition between adjacent items.");
                }
            }
            else
            {
                VerifyListDefinition(document, actual, marker);
                activeListId = marker.ListId;
                activeDefinitionId = actual.DefinitionId;
            }
            verified++;
        }
        if (verified != plan.Summary.ListItems)
        {
            throw new InvalidOperationException(
                $"Expected {plan.Summary.ListItems} verified native list items, observed {verified}.");
        }
    }

    private static void VerifyListDefinition(
        XDocument document,
        SavedNativeList actual,
        PreviewListMarker marker)
    {
        var definitionName = marker.Kind == "bullet" ? "BULLET" : "NUMBERING";
        var definitions = document.Descendants()
            .Where(element => element.Name.LocalName == definitionName)
            .Where(element => int.TryParse(element.Attribute("Id")?.Value, out var id) &&
                id == actual.DefinitionId)
            .ToArray();
        if (definitions.Length != 1)
        {
            throw new InvalidOperationException(
                $"Expected one native {definitionName} definition {actual.DefinitionId}, " +
                $"found {definitions.Length}.");
        }
        if (marker.Kind == "bullet")
        {
            return;
        }

        var definition = definitions[0];
        var level = definition.Elements()
            .Where(element => element.Name.LocalName == "PARAHEAD")
            .SingleOrDefault(element =>
                int.TryParse(element.Attribute("Level")?.Value, out var parsedLevel) &&
                parsedLevel == marker.Depth + 1);
        if (!int.TryParse(definition.Attribute("Start")?.Value, out var start) ||
            start != marker.Number ||
            level is null ||
            !int.TryParse(level.Attribute("Start")?.Value, out var levelStart) ||
            levelStart != marker.Number ||
            !string.Equals(level.Attribute("NumFormat")?.Value, "Digit", StringComparison.Ordinal) ||
            !string.Equals(level.Value, $"^{marker.Depth + 1}.", StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                $"Native ordered list {marker.ListId} did not start at " +
                $"{marker.Number} for depth {marker.Depth}.");
        }
    }

    private static IEnumerable<(string Symbolic, string Text)> StyledTexts(
        IrPreviewPlan plan,
        InvestigationTemplateProfile profile)
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
                            ? profile.Figure.SourceLabel
                            : $"{profile.Figure.SourceLabel} {operation.Lines[2]}");
                    break;
                default:
                    throw new InvalidOperationException(
                        $"Unknown preview operation during text verification: {operation.Kind}");
            }
        }
    }

    private static IEnumerable<ExpectedParagraph> ExpectedParagraphs(
        IrPreviewPlan plan,
        InvestigationTemplateProfile profile)
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
                            index == 0 ? operation.ListMarker : null,
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
                        null,
                        null);
                    break;
                case "figure":
                    yield return new ExpectedParagraph(
                        "figure",
                        string.Empty,
                        true,
                        false,
                        0,
                        null,
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
                        null,
                        PreviewRunBuilder.Coalesce(captionRuns));
                    var sourceRuns = new List<PreviewTextRun>
                    {
                        new(
                            operation.Lines[2].Length == 0
                                ? profile.Figure.SourceLabel
                                : profile.Figure.SourceLabel + " ",
                            false,
                            false),
                    };
                    sourceRuns.AddRange(FormattedLine(operation, 2));
                    yield return new ExpectedParagraph(
                        "figure.source",
                        operation.Lines[2].Length == 0
                            ? profile.Figure.SourceLabel
                            : $"{profile.Figure.SourceLabel} {operation.Lines[2]}",
                        false,
                        false,
                        0,
                        null,
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

    private static int CountNativeListParagraphs(dynamic hwp)
    {
        IReadOnlyList<SavedParagraph> paragraphs = ReadParagraphs(hwp);
        return paragraphs.Count(paragraph => paragraph.NativeList is not null);
    }

    private static IReadOnlyList<SavedParagraph> ReadParagraphs(dynamic hwp)
    {
        var document = XDocument.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        var characterShapes = HwpmlCharacterShapes.Read(document);
        var paragraphShapes = document.Descendants()
            .Where(element => element.Name.LocalName == "PARASHAPE")
            .Where(element => int.TryParse(element.Attribute("Id")?.Value, out _))
            .ToDictionary(
                element => int.Parse(element.Attribute("Id")!.Value),
                element => element);
        return document.Descendants()
            .Where(element => element.Name.LocalName == "SECTION")
            .SelectMany(section => section.Elements()
                .Where(element => element.Name.LocalName == "P"))
            .Select(element =>
            {
                var paragraphShapeId = int.TryParse(
                    element.Attribute("ParaShape")?.Value,
                    out var parsedParagraphShapeId)
                    ? parsedParagraphShapeId
                    : -1;
                paragraphShapes.TryGetValue(paragraphShapeId, out var paragraphShape);
                SavedNativeList? nativeList = null;
                var headingType = paragraphShape?.Attribute("HeadingType")?.Value;
                if (headingType is "Bullet" or "Number" &&
                    int.TryParse(paragraphShape?.Attribute("Level")?.Value, out var level) &&
                    int.TryParse(paragraphShape?.Attribute("Heading")?.Value, out var definitionId) &&
                    int.TryParse(
                        paragraphShape?.Elements()
                            .SingleOrDefault(child => child.Name.LocalName == "PARAMARGIN")?
                            .Attribute("Left")?.Value,
                        out var leftMargin))
                {
                    nativeList = new SavedNativeList(
                        headingType == "Bullet" ? "bullet" : "ordered",
                        level,
                        definitionId,
                        leftMargin);
                }
                return new SavedParagraph(
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
                    nativeList,
                    ReadSavedRuns(element, characterShapes));
            })
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
    bool BaseItalic,
    int BaseLeftMargin);

internal sealed class AuriPreviewStyleBindings
{
    private readonly IReadOnlyDictionary<string, NativeStyle> styles;

    private AuriPreviewStyleBindings(
        InvestigationTemplateProfile profile,
        IReadOnlyDictionary<string, NativeStyle> styles,
        NativeStyle resetStyle)
    {
        Profile = profile;
        this.styles = styles;
        ResetStyle = resetStyle;
        Bindings = profile.ParagraphStyles
            .Select(required => new StyleBinding(
                required.Symbolic,
                required.NativeName,
                styles[required.Symbolic].Id))
            .ToArray();
    }

    public IReadOnlyList<StyleBinding> Bindings { get; }

    public InvestigationTemplateProfile Profile { get; }

    public NativeStyle ResetStyle { get; }

    public static AuriPreviewStyleBindings Bind(
        dynamic hwp,
        InvestigationTemplateProfile profile)
    {
        var document = XDocument.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        return BindDocument(document, profile);
    }

    public static AuriPreviewStyleBindings BindDocument(
        XDocument document,
        InvestigationTemplateProfile profile)
    {
        var styleElements = document.Descendants()
            .Where(element => element.Name.LocalName == "STYLE")
            .ToArray();
        var characterShapes = HwpmlCharacterShapes.Read(document);
        var paragraphShapes = document.Descendants()
            .Where(element => element.Name.LocalName == "PARASHAPE")
            .Where(element => int.TryParse(element.Attribute("Id")?.Value, out _))
            .ToDictionary(
                element => int.Parse(element.Attribute("Id")!.Value),
                element => element);
        var bindings = new Dictionary<string, NativeStyle>(StringComparer.Ordinal);

        foreach (var required in profile.ParagraphStyles)
        {
            bindings.Add(
                required.Symbolic,
                BindNativeStyle(
                    styleElements,
                    characterShapes,
                    paragraphShapes,
                    required.NativeName));
        }

        var resetStyle = BindNativeStyle(
            styleElements,
            characterShapes,
            paragraphShapes,
            profile.ResetNativeStyle);
        return new AuriPreviewStyleBindings(profile, bindings, resetStyle);
    }

    private static NativeStyle BindNativeStyle(
        IReadOnlyList<XElement> styleElements,
        IReadOnlyDictionary<int, CharacterMarks> characterShapes,
        IReadOnlyDictionary<int, XElement> paragraphShapes,
        string nativeName)
    {
        var matches = styleElements.Where(element =>
                string.Equals(element.Attribute("Type")?.Value, "Para", StringComparison.Ordinal) &&
                string.Equals(element.Attribute("Name")?.Value, nativeName, StringComparison.Ordinal))
            .ToArray();
        if (matches.Length != 1 ||
            !int.TryParse(matches[0].Attribute("Id")?.Value, out var nativeId) ||
            !int.TryParse(matches[0].Attribute("CharShape")?.Value, out var characterShapeId) ||
            !characterShapes.TryGetValue(characterShapeId, out var characterMarks) ||
            !int.TryParse(matches[0].Attribute("ParaShape")?.Value, out var paragraphShapeId) ||
            !paragraphShapes.TryGetValue(paragraphShapeId, out var paragraphShape) ||
            !int.TryParse(
                paragraphShape.Elements()
                    .SingleOrDefault(child => child.Name.LocalName == "PARAMARGIN")?
                    .Attribute("Left")?.Value,
                out var baseLeftMargin))
        {
            throw new InvalidOperationException(
                $"Expected one complete AURI paragraph style named {nativeName}, found {matches.Length}.");
        }
        return new NativeStyle(
            nativeId,
            nativeName,
            characterMarks.Bold,
            characterMarks.Italic,
            baseLeftMargin);
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
