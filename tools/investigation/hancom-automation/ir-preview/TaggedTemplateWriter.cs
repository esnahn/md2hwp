using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TaggedTemplateResult(string Output, string Contract, bool Reopened, bool SourceUnchanged);

internal static partial class HancomPreviewWriter
{
    public static TaggedTemplateResult AuthorTaggedTemplate(string templatePath, string outputPath,
        string repositoryRoot, bool visible)
    {
        var source = ValidateTemplate(templatePath);
        var output = ValidateNewHwpPath(outputPath, "Tagged template");
        var hash = HashFile(source);
        if (hash != "5CAEABF6C3BF1EE10B68FF810678374B0C775CB6A27B423F5326F3ED97080D55")
            throw new InvalidDataException("Authoring is restricted to the investigated minimal.hwp; rendering is structurally bound.");
        EnsureInteractiveContext(); EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(repositoryRoot);
        var temporary = Path.Combine(Path.GetDirectoryName(output)!, $".tagged-author-{Guid.NewGuid():N}.hwp");
        try
        {
            File.Copy(source, temporary, false);
            WithHwp(module, hwp =>
            {
                Open(hwp, temporary, visible);
                XElement[] original = RangeRoots(hwp);
                if (original.Length != 8 || TaggedTemplateBinding.DirectText(original[2]) != "AURI 기본연구보고서 변환 예시")
                    throw new InvalidDataException("Unexpected minimal content layout.");
                var prefix = original.Take(2).ToArray();
                XDocument originalDocument = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                var styles = originalDocument.Descendants().Where(e => e.Name.LocalName == "STYLE" && (string?)e.Attribute("Type") == "Para")
                    .ToDictionary(e => e.Attribute("Name")!.Value, e => int.Parse(e.Attribute("Id")!.Value));
                int body = styles["본문"], reset = styles["바탕글"];
                // Coordinates below are checked against the exact source fixture only.
                ReplaceTaggedRootText(hwp, 2, TaggedTemplateBinding.Tag("heading.2"));
                ReplaceTaggedRootText(hwp, 3, TaggedTemplateBinding.Tag("body"));
                MoveTaggedRoot(hwp, 4);
                FindTaggedText(hwp, "스타일 블록 예시:");
                Run(hwp, "MoveParaBegin"); Run(hwp, "MoveSelParaEnd");
                InsertText(hwp, TaggedTemplateBinding.Tag("slot:box.content"));
                MoveTaggedRoot(hwp, 5);
                FindTaggedText(hwp, "스타일 대응 예시");
                InsertText(hwp, TaggedTemplateBinding.Tag("slot:figure.caption"));
                DeleteRangeParagraphs(hwp, 6, 7);
                ReplaceTaggedRootText(hwp, 6, TaggedTemplateBinding.Tag("slot:figure.source"));
                // Source was the last paragraph; establish a successor before ranges.
                Run(hwp, "MoveDocEnd"); Run(hwp, "BreakPara");
                InsertTaggedBefore(hwp, 5, TaggedTemplateBinding.Tag("slot:figure.image"), body);
                int figureSource = FindTaggedRoot(hwp, TaggedTemplateBinding.Tag("slot:figure.source"));
                InsertTaggedBefore(hwp, figureSource + 1, TaggedTemplateBinding.Tag("end:figure"), reset);
                InsertTaggedBefore(hwp, FindTaggedRoot(hwp, TaggedTemplateBinding.Tag("slot:figure.image")), TaggedTemplateBinding.Tag("begin:figure"), reset);
                InsertTaggedBefore(hwp, 5, TaggedTemplateBinding.Tag("end:block.box"), reset);
                InsertTaggedBefore(hwp, 4, TaggedTemplateBinding.Tag("begin:block.box"), reset);
                InsertTaggedBefore(hwp, 2, TaggedTemplateBinding.Tag("begin:samples"), reset);
                void Append(string text, int style)
                {
                    Run(hwp, "MoveDocEnd");
                    ApplyTaggedStyleId(hwp, style);
                    InsertText(hwp, text); Run(hwp, "BreakPara");
                }
                Append(TaggedTemplateBinding.Tag("contract:minimal-1"), reset);
                Append(TaggedTemplateBinding.Tag("figure.max-width-mm:142"), reset);
                Append(TaggedTemplateBinding.Tag("lists.max-depth:6"), reset);
                Append(TaggedTemplateBinding.Tag("lists.indent-hwp:2000"), reset);
                Append(TaggedTemplateBinding.Tag("source-label:출처:"), reset);
                Append(TaggedTemplateBinding.Tag("reset"), reset);
                foreach (var (role, name) in new[] {
                    ("heading.1", "장제목 (개요 1)"), ("heading.3", "1) (개요 3)"),
                    ("heading.4", "① (개요 4)"), ("heading.5", "□ (개요 5)"), ("heading.6", "․ (개요 6)") })
                    Append(TaggedTemplateBinding.Tag(role), styles[name]);
                Append(TaggedTemplateBinding.Tag("end:samples"), reset);
                Append(TaggedTemplateBinding.Tag("content"), body);
                Run(hwp, "FileSave"); CloseDocument(hwp); Open(hwp, temporary, visible);
                XDocument reopened = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                var binding = TaggedTemplateBinding.Read(reopened, temporary);
                TemplateRangeStructure.RequireOriginalStyleDefinitions(originalDocument, reopened, prefix);
                var resolved = AuriPreviewStyleBindings.BindDocument(reopened, binding.Profile);
                _ = AuriMinimalBoxPrototype.Bind(hwp, resolved);
                _ = AuriMinimalCaptionPrototype.Bind(hwp, resolved);
                XElement[] roots = RangeRoots(hwp);
                if (binding.SamplesBegin != 2 || !TemplateRangeStructure.Equivalent(prefix, roots.Take(2).ToArray()))
                    throw new InvalidOperationException("Authoring altered the static cover/header content.");
                return true;
            });
            if (HashFile(source) != hash) throw new InvalidOperationException("Source template changed.");
            File.Move(temporary, output);
            return new(output, "minimal-1", true, true);
        }
        finally { if (File.Exists(temporary)) File.Delete(temporary); }
    }

    public static TaggedTemplateResult RenderTaggedTemplate(string irPath, string templatePath,
        string outputPath, string repositoryRoot, bool visible)
    {
        var source = ValidateTemplate(templatePath);
        var output = ValidateRenderedOutput(outputPath, source, irPath);
        var hash = HashFile(source);
        if (!File.Exists(irPath)) throw new FileNotFoundException("Missing IR input.", irPath);
        EnsureInteractiveContext(); EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(repositoryRoot);
        var temporary = Path.Combine(Path.GetDirectoryName(output)!, $".tagged-render-{Guid.NewGuid():N}.hwp");
        try
        {
            File.Copy(source, temporary, false);
            WithHwp(module, hwp =>
            {
                Open(hwp, temporary, visible);
                XDocument document = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                var binding = TaggedTemplateBinding.Read(document, temporary);
                var profile = binding.Profile;
                var plan = IrPreviewPlan.Load(irPath, repositoryRoot, profile);
                var styles = AuriPreviewStyleBindings.BindDocument(document, profile);
                var rootsBefore = AuriMinimalBoxPrototype.RootParagraphs(document);
                var prefix = rootsBefore.Take(binding.SamplesBegin).ToArray();
                var boundBox = AuriMinimalBoxPrototype.Bind(hwp, styles);
                var boundCaption = AuriMinimalCaptionPrototype.Bind(hwp, styles);
                var box = plan.Summary.BoxOperations > 0 ? boundBox : null;
                var caption = plan.Summary.FigureOperations > 0 ? boundCaption : null;
                int start = PrepareInsertionTarget(hwp, profile);

                int? list = null;
                foreach (var operation in plan.Operations)
                {
                    Console.Error.WriteLine($"render-tagged: {operation.Kind}/{operation.Label}");
                    list = RenderOperation(hwp, operation, styles, box, caption, list);
                }
                ClearNativeListAtCaret(hwp, styles.Resolve("body"));
                ApplyResolvedParagraphStyle(hwp, styles, styles.Resolve("body"));
                RemoveHyperlinksInRoots(hwp, start, ((XElement[])RangeRoots(hwp)).Length);
                VerifyStyles(hwp, plan, styles, start);
                VerifyCharacterMarks(hwp, plan, styles, start);
                VerifyBoxes(hwp, plan, styles, box, start);
                VerifyCaptions(hwp, plan, styles, caption, start);
                VerifyLists(hwp, plan, profile, start);
                RequireNoHyperlinks(((XElement[])RangeRoots(hwp)).Skip(start));
                // All clones are verified before removing the original prototypes.
                DeleteRangeParagraphs(hwp, binding.SamplesBegin, binding.SamplesEnd + 1);
                start -= binding.SamplesEnd + 1 - binding.SamplesBegin;
                Run(hwp, "FileSave"); CloseDocument(hwp); Open(hwp, temporary, visible);
                VerifyText(hwp, plan, profile);
                VerifyStyles(hwp, plan, styles, start);
                VerifyCharacterMarks(hwp, plan, styles, start);
                VerifyBoxes(hwp, plan, styles, box, start, verifyPrototype: false);
                VerifyCaptions(hwp, plan, styles, caption, start, verifyPrototype: false);
                VerifyLists(hwp, plan, profile, start);
                XElement[] finalRoots = RangeRoots(hwp);
                RequireNoHyperlinks(finalRoots.Skip(start));
                XDocument finalDocument = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                TemplateRangeStructure.RequireOriginalStyleDefinitions(document, finalDocument, prefix);
                if (!TemplateRangeStructure.Equivalent(prefix, finalRoots.Take(prefix.Length).ToArray(), document, finalDocument))
                    throw new InvalidOperationException("Rendering changed static cover/header content: " +
                        TemplateRangeStructure.DescribeDifference(prefix, finalRoots.Take(prefix.Length).ToArray()));
                var expected = ExpectedParagraphs(plan, profile).ToArray();
                if (finalRoots.Length != prefix.Length + expected.Length + 1 || !IsSimpleParagraph(finalRoots[^1], ""))
                    throw new InvalidOperationException("Unexpected leftover samples or generated paragraph count.");
                IReadOnlyList<SavedParagraph> saved = ReadParagraphs(hwp);
                if (saved[^1].NativeList is not null ||
                    (string?)finalRoots[^1].Attribute("Style") != styles.Resolve("body").Id.ToString())
                    throw new InvalidOperationException("The empty terminal paragraph inherited a heading or list marker.");
                VerifyTaggedLineBreaks(finalRoots.Skip(start).ToArray(), plan);
                VerifyTaggedPictures(finalRoots.Skip(start).ToArray(), plan);
                if (finalRoots.SelectMany(p => p.Descendants()).Count(e => e.Name.LocalName == "PICTURE") !=
                    prefix.SelectMany(p => p.Descendants()).Count(e => e.Name.LocalName == "PICTURE") + plan.Summary.FigureOperations)
                    throw new InvalidOperationException("Unexpected generated picture count.");
                if (finalRoots.SelectMany(p => p.Descendants()).Count(e => e.Name.LocalName == "AUTONUM" && (string?)e.Attribute("NumberType") == "Figure") !=
                    prefix.SelectMany(p => p.Descendants()).Count(e => e.Name.LocalName == "AUTONUM" && (string?)e.Attribute("NumberType") == "Figure") + plan.Summary.FigureOperations)
                    throw new InvalidOperationException("Unexpected generated caption count.");
                return true;
            });
            if (HashFile(source) != hash) throw new InvalidOperationException("Source template changed.");
            PublishRenderedOutput(temporary, output);
            return new(output, "minimal-1", true, true);
        }
        finally { if (File.Exists(temporary)) File.Delete(temporary); }
    }

    internal static string ValidateRenderedOutput(string outputPath, params string[] protectedPaths)
    {
        var output = Path.GetFullPath(outputPath);
        if (!string.Equals(Path.GetExtension(output), ".hwp", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Rendered document must preserve HWP format.");
        if (Directory.Exists(output)) throw new IOException($"Output is a directory: {output}");
        static string Identity(string path)
        {
            var full = Path.GetFullPath(path);
            var parent = Path.GetDirectoryName(full);
            if (parent is null) return full;
            FileSystemInfo info = Directory.Exists(full) ? new DirectoryInfo(full) : new FileInfo(full);
            var target = info.ResolveLinkTarget(true);
            return target is not null ? Identity(target.FullName) : Path.Combine(Identity(parent), Path.GetFileName(full));
        }
        foreach (var path in protectedPaths)
        {
            if (string.Equals(output, Path.GetFullPath(path), StringComparison.OrdinalIgnoreCase) ||
                (File.Exists(output) && File.Exists(path) && string.Equals(Identity(output), Identity(path), StringComparison.OrdinalIgnoreCase)))
                throw new IOException($"Output must not replace input: {path}");
        }
        if (!Directory.Exists(Path.GetDirectoryName(output))) throw new DirectoryNotFoundException($"Missing output directory: {output}");
        return output;
    }

    internal static void PublishRenderedOutput(string temporary, string output) => File.Move(temporary, output, overwrite: true);

    private static void VerifyTaggedLineBreaks(IReadOnlyList<XElement> roots, IrPreviewPlan plan)
    {
        var index = 0;
        foreach (var operation in plan.Operations)
        {
            if (operation.Kind == "text" && roots[index].Descendants().Count(e => e.Name.LocalName == "LINEBREAK") != operation.Lines.Count - 1)
                throw new InvalidOperationException("IR line breaks did not remain within their paragraph.");
            index += operation.Kind == "figure" ? 3 : 1;
        }
    }

    private static void VerifyTaggedPictures(IReadOnlyList<XElement> roots, IrPreviewPlan plan)
    {
        var index = 0;
        foreach (var operation in plan.Operations)
        {
            if (operation.Kind == "figure")
            {
                var picture = roots[index].Descendants().Single(e => e.Name.LocalName == "PICTURE");
                var shape = picture.Elements().Single(e => e.Name.LocalName == "SHAPEOBJECT");
                var size = shape.Elements().Single(e => e.Name.LocalName == "SIZE");
                var position = shape.Elements().Single(e => e.Name.LocalName == "POSITION");
                var width = double.Parse(size.Attribute("Width")!.Value, System.Globalization.CultureInfo.InvariantCulture);
                var height = double.Parse(size.Attribute("Height")!.Value, System.Globalization.CultureInfo.InvariantCulture);
                const double unitsPerMillimeter = 7200.0 / 25.4;
                if ((string?)position.Attribute("TreatAsChar") != "true" ||
                    Math.Abs(width - operation.ImageWidthMillimeters!.Value * unitsPerMillimeter) > 2 ||
                    Math.Abs(height - operation.ImageHeightMillimeters!.Value * unitsPerMillimeter) > 2)
                    throw new InvalidOperationException("Saved figure placement or aspect-preserving dimensions differ from the plan.");
            }
            index += operation.Kind == "figure" ? 3 : 1;
        }
    }

    private static void MoveTaggedRoot(dynamic hwp, int index)
    {
        if (!(bool)hwp.SetPos(0, index, 0)) throw new InvalidOperationException($"Cannot move to root[{index}].");
    }
    private static void ApplyTaggedStyleId(dynamic hwp, int id)
    {
        _ = hwp.HAction.GetDefault("Style", hwp.HParameterSet.HStyle.HSet);
        if ((int)hwp.HParameterSet.HStyle.Apply == id)
        {
            // Exact minimal-fixture authoring only: IDs 0/1 were checked above.
            _ = hwp.HAction.GetDefault("StyleEx", hwp.HParameterSet.HStyle.HSet);
            hwp.HParameterSet.HStyle.Apply = id == 0 ? 1 : 0;
            object? detour = hwp.HAction.Execute("StyleEx", hwp.HParameterSet.HStyle.HSet);
            if (!IndicatesSuccess(detour)) throw new InvalidOperationException("Cannot apply authoring style detour.");
        }
        _ = hwp.HAction.GetDefault("StyleEx", hwp.HParameterSet.HStyle.HSet);
        hwp.HParameterSet.HStyle.Apply = id;
        object? applied = hwp.HAction.Execute("StyleEx", hwp.HParameterSet.HStyle.HSet);
        if (!IndicatesSuccess(applied)) throw new InvalidOperationException($"Cannot apply style {id}.");
    }
    private static void ReplaceTaggedRootText(dynamic hwp, int index, string text)
    {
        MoveTaggedRoot(hwp, index); Run(hwp, "MoveParaBegin"); Run(hwp, "MoveSelParaEnd");
        InsertText(hwp, text);
    }
    private static void InsertTaggedBefore(dynamic hwp, int index, string text, int style)
    {
        MoveTaggedRoot(hwp, index); Run(hwp, "MoveParaBegin"); Run(hwp, "BreakPara");
        MoveTaggedRoot(hwp, index); ApplyTaggedStyleId(hwp, style); InsertText(hwp, text);
    }
    private static int FindTaggedRoot(dynamic hwp, string text)
    {
        XElement[] roots = RangeRoots(hwp);
        var matches = roots.Select((p, i) => (p, i)).Where(x => TaggedTemplateBinding.DirectText(x.p) == text).ToArray();
        if (matches.Length != 1) throw new InvalidOperationException($"Expected unique tagged root {text}.");
        return matches[0].i;
    }
    private static void FindTaggedText(dynamic hwp, string text)
    {
        _ = hwp.HAction.GetDefault("RepeatFind", hwp.HParameterSet.HFindReplace.HSet);
        hwp.HParameterSet.HFindReplace.FindString = text;
        hwp.HParameterSet.HFindReplace.Direction = hwp.FindDir("Forward");
        hwp.HParameterSet.HFindReplace.IgnoreMessage = 1;
        if (!(bool)hwp.HAction.Execute("RepeatFind", hwp.HParameterSet.HFindReplace.HSet))
            throw new InvalidOperationException($"Cannot find authored sample text: {text}");
    }
}
