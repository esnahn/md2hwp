using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
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

internal sealed record TemplatePairPreparationResult(
    string Template,
    string Baseline,
    string Marked,
    string Marker,
    string TemplateSha256,
    string BaselineSha256,
    string MarkedSha256,
    int MarkerParagraphs,
    bool BaselineByteIdentical,
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
    IReadOnlyList<SavedTextRun> Runs,
    int LeftMargin = 0,
    int Indentation = 0);

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
    IReadOnlyList<PreviewTextRun>? FormattedRuns,
    PreviewListMarker? ListContinuation = null);

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

internal static partial class HancomPreviewWriter
{
    private const string ProgId = "HWPFrame.HwpObject";
    private const string ModuleName = "FilePathCheckerModuleExample";
    private const string OpenOptions = "lock:false;forceopen:true;suspendpassword:true;versionwarning:false";
    private const string ComparisonMarker = "{{MD2HWP_INSERTION_TARGET_V0_1}}";

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
                throw SecurityModuleRegistration.Unavailable("한글이 등록된 보안 모듈을 받아들이지 않았습니다.");
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

    private static string ValidateNewHwpPath(string path, string label)
    {
        var fullPath = Path.GetFullPath(path);
        if (!string.Equals(Path.GetExtension(fullPath), ".hwp", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException($"{label} must preserve the HWP format: {fullPath}");
        }
        if (File.Exists(fullPath) || Directory.Exists(fullPath))
        {
            throw new IOException($"{label} path already exists: {fullPath}");
        }
        var parent = Path.GetDirectoryName(fullPath)!;
        if (!Directory.Exists(parent))
        {
            throw new DirectoryNotFoundException($"Missing {label.ToLowerInvariant()} directory: {parent}");
        }
        return fullPath;
    }

    private static int CountMarkerParagraphs(XDocument document, string marker) =>
        document.Descendants()
            .Where(element => element.Name.LocalName == "SECTION")
            .Sum(section =>
            {
                var roots = section.Elements()
                    .Where(element => element.Name.LocalName == "P")
                    .ToArray();
                return roots.Select((paragraph, index) => (paragraph, index))
                    .Count(candidate =>
                        candidate.index + 1 == roots.Length - 1 &&
                        IsSimpleParagraph(candidate.paragraph, marker) &&
                        IsSimpleParagraph(roots[candidate.index + 1], string.Empty));
            });

    private static int PrepareInsertionTarget(
        dynamic hwp,
        InvestigationTemplateProfile profile)
    {
        if (string.Equals(
                profile.InsertionTarget.Kind,
                "document_end",
                StringComparison.Ordinal))
        {
            var paragraphsBefore = ReadParagraphs(hwp).Count;
            Run(hwp, "MoveDocEnd");
            Run(hwp, "BreakPara");
            return paragraphsBefore;
        }

        var marker = profile.InsertionTarget.Marker ??
            throw new InvalidOperationException("The marker insertion target has no marker text.");
        var beforeDocument = RenderProfile.ReadDocument((object)hwp);
        var beforeRoots = beforeDocument.Descendants()
            .Where(element => element.Name.LocalName == "SECTION")
            .SelectMany(section => section.Elements()
                .Where(element => element.Name.LocalName == "P"))
            .ToArray();
        var candidates = beforeRoots
            .Select((paragraph, index) => (paragraph, index))
            .Where(candidate =>
                candidate.index + 1 == beforeRoots.Length - 1 &&
                IsSimpleParagraph(candidate.paragraph, marker) &&
                IsSimpleParagraph(beforeRoots[candidate.index + 1], string.Empty))
            .ToArray();
        if (candidates.Length != 1)
        {
            throw new InvalidOperationException(
                $"Expected one dedicated end marker with an empty successor, observed {candidates.Length}.");
        }

        var beforeText = beforeRoots.Select(root => root.Value).ToArray();
        var markerIndex = candidates[0].index;
        if (!(bool)hwp.SetPos(0, markerIndex, 0))
        {
            throw new InvalidOperationException(
                $"Hancom could not move to marker root paragraph {markerIndex}.");
        }
        Run(hwp, "MoveParaBegin");
        Run(hwp, "MoveSelNextParaBegin");
        Run(hwp, "Delete");

        var afterDocument = RenderProfile.ReadDocument((object)hwp);
        var afterRoots = afterDocument.Descendants()
            .Where(element => element.Name.LocalName == "SECTION")
            .SelectMany(section => section.Elements()
                .Where(element => element.Name.LocalName == "P"))
            .ToArray();
        var expectedText = beforeText.Where((_, index) => index != markerIndex).ToArray();
        var afterText = afterRoots.Select(root => root.Value).ToArray();
        if (!afterText.SequenceEqual(expectedText, StringComparer.Ordinal) ||
            afterRoots.Length == 0 ||
            !IsSimpleParagraph(afterRoots[^1], string.Empty) ||
            afterRoots.Any(root => root.Value.Contains(marker, StringComparison.Ordinal)))
        {
            throw new InvalidOperationException(
                "Deleting the insertion-marker paragraph changed unexpected document text or structure.");
        }

        Run(hwp, "MoveDocEnd");
        return afterRoots.Length - 1;
    }

    private static bool IsSimpleParagraph(XElement paragraph, string text) =>
        string.Equals(paragraph.Value, text, StringComparison.Ordinal) &&
        !paragraph.Descendants().Any(element =>
            element.Name.LocalName is "P" or "TABLE" or "PICTURE" or "AUTONUM");

    private static void InsertComparisonMarker(dynamic hwp, string marker)
    {
        _ = hwp.HAction.GetDefault("InsertText", hwp.HParameterSet.HInsertText.HSet);
        hwp.HParameterSet.HInsertText.Text = marker;
        if (!(bool)hwp.HAction.Execute("InsertText", hwp.HParameterSet.HInsertText.HSet))
        {
            throw new InvalidOperationException("Hancom could not insert the comparison marker.");
        }
    }

    private static int? RenderOperation(
        dynamic hwp,
        PreviewOperation operation,
        AuriPreviewStyleBindings styles,
        AuriMinimalBoxPrototype? boxPrototype,
        AuriMinimalCaptionPrototype? captionPrototype,
        FigureSourcePrototype figureSource,
        int? activeListId,
        NativeListContinuations continuations)
    {
        if (operation.Kind is "text" or "table")
        {
            var style = styles.Resolve(operation.ParagraphStyle ??
                throw new InvalidOperationException($"Missing paragraph style for {operation.Label}."));
            var marker = operation.ListMarker;
            for (var index = 0; index < operation.Lines.Count; index++)
            {
                if (index > 0 && styles.Profile.PreserveParagraphLineBreaks)
                {
                    Run(hwp, "BreakLine");
                }
                else if (index == 0 && marker is not null)
                {
                    if (activeListId != marker.ListId)
                    {
                        if (activeListId is null)
                        {
                            ApplyResolvedParagraphStyle(hwp, styles, style);
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
                        ApplyResolvedParagraphStyle(hwp, styles, style);
                    }
                }
                if (operation.ListContinuation is { } continuation)
                    continuations.Apply(hwp, continuation);
                InsertFormattedLine(hwp, FormattedLine(operation, index), style);
                if (marker is not null && index == operation.Lines.Count - 1)
                    continuations.Record(hwp, marker, styles.Profile.Lists);
                if (!styles.Profile.PreserveParagraphLineBreaks || index == operation.Lines.Count - 1)
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
            _ = boxPrototype.Insert(hwp, styles, operation.Lines, operation.SourceRuns);
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
            ApplyResolvedParagraphStyle(hwp, styles, figureStyle);
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
        if (operation.Lines[2].Length > 0)
            figureSource.Insert(hwp, styles, FormattedLine(operation, 2));
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
        ApplyResolvedParagraphStyle(hwp, styles, targetStyle);
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
        ClearNativeListAtCaret(hwp, bodyStyle);
        listLayout.Prototype(marker.Kind).Apply(hwp, marker, bodyStyle, listLayout);
        if (marker.Kind == "bullet") return;
        _ = hwp.HAction.GetDefault("ParagraphShape", hwp.HParameterSet.HParaShape.HSet);
        if (marker.Kind == "ordered")
        {
            hwp.HParameterSet.HParaShape.HeadingType = 2;
            hwp.HParameterSet.HParaShape.Level = marker.Depth;
            hwp.HParameterSet.HParaShape.Numbering.NewList = 1;
            hwp.HParameterSet.HParaShape.Numbering.StartNumber = marker.Number;
            SetNativeListLevelStart(hwp.HParameterSet.HParaShape.Numbering, marker.Depth, marker.Number);
            // Reapply the template level format alongside the heading/level and new start.
            SetNativeListLevelNumberFormat(hwp.HParameterSet.HParaShape.Numbering, marker.Depth,
                listLayout.Prototype("ordered").NumberFormat(marker.Depth),
                listLayout.Prototype("ordered").StringFormat(marker.Depth));
        }
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

    private static void SetNativeListLevelNumberFormat(dynamic numbering, int depth, ushort format, string pattern)
    {
        switch (depth)
        {
            case 0: numbering.NumFormatLevel0 = format; numbering.StrFormatLevel0 = pattern; break;
            case 1: numbering.NumFormatLevel1 = format; numbering.StrFormatLevel1 = pattern; break;
            case 2: numbering.NumFormatLevel2 = format; numbering.StrFormatLevel2 = pattern; break;
            case 3: numbering.NumFormatLevel3 = format; numbering.StrFormatLevel3 = pattern; break;
            case 4: numbering.NumFormatLevel4 = format; numbering.StrFormatLevel4 = pattern; break;
            case 5: numbering.NumFormatLevel5 = format; numbering.StrFormatLevel5 = pattern; break;
            case 6: numbering.NumFormatLevel6 = format; numbering.StrFormatLevel6 = pattern; break;
            default: throw new InvalidOperationException($"Unsupported native list depth: {depth}");
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

    private static void ApplyResolvedParagraphStyle(dynamic hwp, AuriPreviewStyleBindings styles, NativeStyle target)
    {
        // Reapplying the modified current style can open a modal in HWP 2020.
        // Keep the distinct-style detour, but sample only the final paragraph.
        // Block exports renumber IDs; compare names and apply original IDs.
        var current = CurrentParagraphStyle.Read((object)hwp);
        if (string.Equals(current, target.Name, StringComparison.Ordinal))
        {
            var detour = target.Id == styles.ResetStyle.Id ? styles.Resolve("body") : styles.ResetStyle;
            if (detour.Id == target.Id) throw new InvalidOperationException("No distinct style detour is bound.");
            ApplyParagraphStyle(hwp, detour);
        }
        ApplyParagraphStyle(hwp, target);
    }

    internal static void InsertText(dynamic hwp, string text)
    {
        _ = hwp.HAction.GetDefault("InsertText", hwp.HParameterSet.HInsertText.HSet);
        hwp.HParameterSet.HInsertText.Text = text;
        if (!(bool)hwp.HAction.Execute("InsertText", hwp.HParameterSet.HInsertText.HSet))
        {
            throw new InvalidOperationException("Hancom failed to insert preview text.");
        }
    }

    internal static void RemoveHyperlinksInRoots(dynamic hwp, int startInclusive, int endExclusive)
    {
        if (startInclusive < 0 || endExclusive < startInclusive)
            throw new ArgumentOutOfRangeException(nameof(startInclusive));
        // Resolve root anchors before deleting any control. A nested box link
        // belongs to the root paragraph containing its newly cloned table.
        var links = new List<object>();
        for (dynamic? control = hwp.HeadCtrl; control is not null; control = control.Next)
        {
            if ((string)control.CtrlID != "%hlk") continue;
            dynamic anchor = control.GetAnchorPos(2);
            if ((int)anchor.Item("List") != 0)
                throw new InvalidOperationException("Hyperlink did not resolve to a root anchor.");
            int paragraph = (int)anchor.Item("Para");
            if (paragraph >= startInclusive && paragraph < endExclusive)
                links.Add((object)control);
        }
        // DeleteCtrl preserves the label and restores its pre-link character
        // shape. Applying a paragraph style alone leaves the hyperlink field.
        foreach (dynamic link in links)
            if (!(bool)hwp.DeleteCtrl(link))
                throw new InvalidOperationException("Hancom failed to remove a generated hyperlink.");
    }

    internal static void RequireNoHyperlinks(IEnumerable<XElement> roots)
    {
        if (roots.SelectMany(root => root.DescendantsAndSelf()).Any(element =>
                element.Name.LocalName is "FIELDBEGIN" or "FIELDEND" &&
                (string?)element.Attribute("Type") == "Hyperlink"))
            throw new InvalidOperationException("Generated content contains an unexpected hyperlink field.");
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
        // TEXT export can substitute Unicode characters (for example © with ⓒ).
        // Verify the saved document's Unicode content through HWPML instead.
        var document = RenderProfile.ReadDocument((object)hwp);
        var extracted = string.Join("\n", document.Descendants()
            .Where(element => element.Name.LocalName == "P").Select(element => element.Value));
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
        int paragraphsBefore,
        NativeListContinuations? continuations = null)
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
            if (expected.ListContinuation is { } continuation && continuations is not null &&
                (actual.NativeList is not null || actual.LeftMargin != continuations.Margin(continuation) || actual.Indentation != 0))
                throw new InvalidOperationException($"Saved list continuation paragraph {index} lost its body alignment.");
            if (actual.Style != nativeStyle.Id ||
                actual.ContainsPicture != expected.ContainsPicture ||
                actual.ContainsTable != expected.ContainsTable ||
                actual.FigureAutoNumbers != expected.FigureAutoNumbers ||
                !NativeListMatches(
                    actual.NativeList,
                    expected.ListMarker,
                    nativeStyle,
                    styles.Profile.Lists,
                    expected.SymbolicStyle.StartsWith("heading", StringComparison.Ordinal)) ||
                !actual.Text.Contains(expected.Text, StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"Saved preview paragraph {index} did not match {expected.SymbolicStyle}/{nativeStyle.Name}: " +
                    $"expected={expected.Text}, actual={actual.Text}, style={actual.Style}/{nativeStyle.Id}, " +
                    $"list={actual.NativeList}, expectedList={expected.ListMarker}, baseMargin={nativeStyle.BaseLeftMargin}, depthIndent={styles.Profile.Lists.DepthIndentHwpUnits}.");
            }
        }
    }

    private static bool NativeListMatches(
        SavedNativeList? actual,
        PreviewListMarker? expected,
        NativeStyle nativeStyle,
        ProfileListLayout listLayout,
        bool allowNativeHeadingDecoration = false) =>
        (actual, expected) switch
        {
            (null, null) => !allowNativeHeadingDecoration || nativeStyle.BaseNativeList is null,
            (not null, null) when allowNativeHeadingDecoration => actual == nativeStyle.BaseNativeList,
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
            if (expected.SymbolicStyle == "figure.caption")
                actualRuns = SliceSavedRuns(actualRuns, styles.Profile.CaptionSelector.CaptionTextOffset, expected.Text.Length);
            if (expected.SymbolicStyle == "figure.source")
                actualRuns = SliceSavedRuns(actualRuns, styles.Profile.FigureSource.Prefix.Length,
                    expected.FormattedRuns.Sum(r => r.Text.Length));
            if (actualRuns.Count != expectedRuns.Count ||
                actualRuns.Where((run, runIndex) => run != expectedRuns[runIndex]).Any())
            {
                throw new InvalidOperationException(
                    $"Saved preview character marks did not match paragraph {index} ({expected.SymbolicStyle}); " +
                    $"expected {DescribeRuns(expectedRuns)}, actual {DescribeRuns(actualRuns)}.");
            }
        }
    }

    private static IReadOnlyList<SavedTextRun> SliceSavedRuns(IReadOnlyList<SavedTextRun> runs, int start, int length)
    {
        var result = new List<SavedTextRun>();
        var position = 0;
        foreach (var run in runs)
        {
            var begin = Math.Max(start, position);
            var end = Math.Min(start + length, position + run.Text.Length);
            if (end > begin) result.Add(run with { Text = run.Text.Substring(begin - position, end - begin) });
            position += run.Text.Length;
        }
        return CoalesceSavedRuns(result);
    }

    internal static void InsertBoxSourceLine(dynamic hwp, AuriPreviewStyleBindings styles,
        IReadOnlyList<PreviewTextRun> sourceRuns)
    {
        InsertFormattedLine(hwp, sourceRuns, styles.Resolve(styles.Profile.BoxSelector.SourceStyle));
    }

    internal static void VerifyBoxSourceRuns(XElement paragraph, AuriPreviewStyleBindings styles,
        IReadOnlyList<PreviewTextRun> sourceRuns)
    {
        var style = styles.Resolve(styles.Profile.BoxSelector.SourceStyle);
        var expected = CoalesceSavedRuns(sourceRuns.Select(run => new SavedTextRun(run.Text,
            style.BaseBold || run.Strong, style.BaseItalic || run.Emphasis)));
        var actual = ReadSavedRuns(paragraph, HwpmlCharacterShapes.Read(paragraph.Document ??
            throw new InvalidOperationException("Source verification requires its owning HWPML document.")));
        actual = SliceSavedRuns(actual, styles.Profile.BoxSource.Prefix.Length, sourceRuns.Sum(r => r.Text.Length));
        if (!expected.SequenceEqual(actual)) throw new InvalidOperationException("Box source character marks differ from IR.");
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
        int paragraphsBefore,
        bool verifyPrototype = true)
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
        if (verifyPrototype) prototype.VerifyOriginal(roots);
        var appended = roots.Skip(paragraphsBefore).ToArray();
        var rootIndex = 0;
        var verified = 0;
        foreach (var operation in plan.Operations)
        {
            switch (operation.Kind)
            {
                case "text":
                case "table":
                    rootIndex += styles.Profile.PreserveParagraphLineBreaks ? 1 : operation.Lines.Count;
                    break;
                case "box":
                    if (rootIndex >= appended.Length)
                    {
                        throw new InvalidOperationException("Saved preview lost an expected box root.");
                    }
                    prototype.VerifyRenderedRoot(
                        appended[rootIndex],
                        styles,
                        operation.Lines,
                        operation.SourceRuns);
                    rootIndex++;
                    verified++;
                    break;
                case "figure":
                    rootIndex += operation.Lines[2].Length > 0 ? 3 : 2;
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
        int paragraphsBefore,
        bool verifyPrototype = true)
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
        if (verifyPrototype) prototype.VerifyOriginal(roots);
        var appended = roots.Skip(paragraphsBefore).ToArray();
        var rootIndex = 0;
        var verified = 0;
        foreach (var operation in plan.Operations)
        {
            switch (operation.Kind)
            {
                case "text":
                case "table":
                    rootIndex += styles.Profile.PreserveParagraphLineBreaks ? 1 : operation.Lines.Count;
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
                    if (operation.Lines[2].Length > 0)
                        styles.Profile.FigureSource.Verify(appended[rootIndex + 2], operation.Lines[2]);
                    rootIndex += operation.Lines[2].Length > 0 ? 3 : 2;
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

        var document = RenderProfile.ReadDocument((object)hwp);
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
                VerifyListDefinition(document, actual, marker, profile.Lists);
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
        PreviewListMarker marker, ProfileListLayout layout)
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
        layout.Prototype(marker.Kind).Verify(definitions[0], marker, document);
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
                case "table":
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
                    if (operation.Lines[2].Length > 0)
                        yield return ("figure.source", profile.FigureSource.Render(operation.Lines[2]));
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
                case "table":
                    var symbolicStyle = operation.ParagraphStyle ??
                        throw new InvalidOperationException(
                            $"Missing paragraph style for {operation.Label}.");
                    if (profile.PreserveParagraphLineBreaks)
                    {
                        yield return new ExpectedParagraph(symbolicStyle,
                            string.Concat(operation.Lines), false, false, 0,
                            operation.ListMarker,
                            PreviewRunBuilder.Coalesce(Enumerable.Range(0, operation.Lines.Count)
                                .SelectMany(index => FormattedLine(operation, index))), operation.ListContinuation);
                        break;
                    }
                    for (var index = 0; index < operation.Lines.Count; index++)
                    {
                        yield return new ExpectedParagraph(
                            symbolicStyle,
                            operation.Lines[index],
                            false,
                            false,
                            0,
                            index == 0 ? operation.ListMarker : null,
                            FormattedLine(operation, index), operation.ListContinuation);
                    }
                    break;
                case "box":
                    yield return new ExpectedParagraph(
                        profile.BoxSelector.RootStyle,
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
                    yield return new ExpectedParagraph(
                        "figure.caption",
                        operation.Lines[1],
                        false,
                        false,
                        1,
                        null,
                        FormattedLine(operation, 1));
                    if (operation.Lines[2].Length > 0)
                        yield return new ExpectedParagraph("figure.source", profile.FigureSource.Render(operation.Lines[2]),
                            false, false, 0, null, FormattedLine(operation, 2));
                    break;
                default:
                    throw new InvalidOperationException(
                        $"Unknown preview operation during paragraph verification: {operation.Kind}");
            }
        }
    }

    private static int CountPictures(dynamic hwp)
    {
        var xml = (string)hwp.GetTextFile("HWPML2X", "");
        return HwpMarkup.Parse(xml)
            .Descendants()
            .Count(element => element.Name.LocalName == "PICTURE");
    }

    private static int CountFigureAutoNumbers(dynamic hwp) =>
        AuriMinimalCaptionPrototype.CountFigureAutoNumbers(
            RenderProfile.ReadDocument((object)hwp));

    private static int CountNativeListParagraphs(dynamic hwp)
    {
        IReadOnlyList<SavedParagraph> paragraphs = ReadParagraphs(hwp);
        return paragraphs.Count(paragraph => paragraph.NativeList is not null);
    }

    private static IReadOnlyList<SavedParagraph> ReadParagraphs(dynamic hwp)
    {
        var document = RenderProfile.ReadDocument((object)hwp);
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
                    ReadSavedRuns(element, characterShapes),
                    (int?)paragraphShape?.Element("PARAMARGIN")?.Attribute("Left") ?? 0,
                    (int?)paragraphShape?.Element("PARAMARGIN")?.Attribute("Indent") ?? 0);
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
            throw new FileNotFoundException($"Missing HWP template: {fullPath}. Use --template to select another file.", fullPath);
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
    int BaseLeftMargin,
    SavedNativeList? BaseNativeList = null);

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
        var document = RenderProfile.ReadDocument((object)hwp);
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
        SavedNativeList? baseNativeList = null;
        var baseHeadingType = paragraphShape.Attribute("HeadingType")?.Value;
        if (baseHeadingType is "Bullet" or "Number")
        {
            if (!int.TryParse(paragraphShape.Attribute("Level")?.Value, out var level) ||
                !int.TryParse(paragraphShape.Attribute("Heading")?.Value, out var definitionId))
                throw new InvalidOperationException($"Incomplete native heading decoration for style {nativeName}.");
            baseNativeList = new(baseHeadingType == "Bullet" ? "bullet" : "ordered", level, definitionId, baseLeftMargin);
        }
        return new NativeStyle(
            nativeId,
            nativeName,
            characterMarks.Bold,
            characterMarks.Italic,
            baseLeftMargin,
            baseNativeList);
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
        // The development lock is not a runtime compatibility gate.
        _ = repositoryRoot;
        using var registryKey = Registry.CurrentUser.OpenSubKey(
            @"Software\HNC\HwpAutomation\Modules",
            writable: false);
        if (registryKey is null || !registryKey.GetValueNames().Contains(ModuleName, StringComparer.Ordinal))
        {
            throw Unavailable("한글 보안 모듈 등록을 찾지 못했습니다.");
        }
        if (registryKey.GetValueKind(ModuleName) is not RegistryValueKind.String)
        {
            throw Unavailable("한글 보안 모듈 등록 값은 REG_SZ여야 합니다.");
        }
        var registeredPath = registryKey.GetValue(
            ModuleName,
            null,
            RegistryValueOptions.DoNotExpandEnvironmentNames) as string;
        return ValidateRegisteredFile(registeredPath);
    }

    internal static SecurityModuleRegistration ValidateRegisteredFile(string? registeredPath)
    {
        if (string.IsNullOrWhiteSpace(registeredPath) || !Path.IsPathFullyQualified(registeredPath))
        {
            throw Unavailable("한글 보안 모듈 등록 경로는 절대 경로여야 합니다.");
        }
        var fullPath = Path.GetFullPath(registeredPath);
        if (!File.Exists(fullPath))
        {
            throw Unavailable($"등록된 한글 보안 모듈 파일이 없습니다: {fullPath}");
        }
        var actualHash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(fullPath)));
        // Observed hash is diagnostic only. RegisterModule must still succeed before Open.
        return new SecurityModuleRegistration(fullPath, actualHash);
    }

    internal static InvalidOperationException Unavailable(string reason) => new($"{reason}\n{InstallationGuide}");

    internal const string InstallationGuide = "한컴 공식 페이지에서 보안모듈(Automation).zip을 내려받아 압축을 풀고, 동봉된 등록 안내를 따라 현재 사용자 계정에 등록하세요.\nhttps://developer.hancom.com/hwpautomation\n등록 이름: FilePathCheckerModuleExample (REG_SZ, DLL 절대 경로). 앱은 보안 모듈을 자동 설치하거나 등록하지 않습니다.";

    private const string ModuleName = "FilePathCheckerModuleExample";
}
