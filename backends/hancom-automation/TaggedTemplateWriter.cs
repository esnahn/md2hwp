using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TaggedTemplateResult(string Output, string IrVersion, bool Reopened, bool SourceUnchanged);

internal static partial class HancomPreviewWriter
{
    public static TaggedTemplateResult RenderTaggedTemplate(string irPath, string templatePath,
        string outputPath, string repositoryRoot, bool visible, bool verbose = false)
    {
        using var timing = RenderProfile.Start();
        var source = ValidateTemplate(templatePath);
        var output = ValidateRenderedOutput(outputPath, source, irPath);
        var hash = HashFile(source);
        if (!File.Exists(irPath)) throw new FileNotFoundException("Missing IR input.", irPath);
        var metadata = TemplateMetadata.Load(irPath);
        EnsureInteractiveContext(); EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(repositoryRoot);
        var temporary = Path.Combine(Path.GetDirectoryName(output)!, $".tagged-render-{Guid.NewGuid():N}.hwp");
        try
        {
            File.Copy(source, temporary, false);
            WithHwp(module, hwp =>
            {
                var prepareTiming = RenderProfile.Measure("phase.prepare");
                Open(hwp, temporary, visible);
                var preparedMetadata = TemplateMetadata.Prepare(RenderProfile.ReadDocument((object)hwp), metadata);
                XDocument document = TemplateHeadingNumbers.Prepare(TaggedTemplateBinding.PreserveBeginSectionSettings(preparedMetadata.Document));
                var footnotes = NativeFootnotes.Bind(document);
                document = footnotes.LowerSample(document);
                var objectSources = TemplateObjectSources.Lower(document);
                var referenceTemplate = TemplateCrossReferences.Lower(objectSources.Document);
                var tables = TemplateTables.Lower(referenceTemplate.Document);
                var headings = TemplateHeadingBlocks.Lower(tables.Document);
                var boxes = TemplateBoxParagraphs.Lower(headings.Document);
                var nativeFigure = NativeFigureCaption.Lower(boxes.Document);
                _ = TaggedTemplateBinding.ReadFlat(nativeFigure.Document, temporary);
                ImportFigureDocument(hwp, TemplateListPrototype.FreezeMarkerFormatting(nativeFigure.Document));
                document = RenderProfile.ReadDocument((object)hwp);
                var binding = TaggedTemplateBinding.ReadFlat(document, temporary);
                var profile = binding.Profile;
                var plan = HeadingReferenceNumbers.Bind(IrPreviewPlan.Load(irPath, repositoryRoot, profile), referenceTemplate.Layout);
                var styles = AuriPreviewStyleBindings.BindDocument(document, profile);
                var rootsBefore = AuriMinimalBoxPrototype.RootParagraphs(document);
                var prefix = rootsBefore.Take(binding.TemplateBegin).ToArray();
                var boundBox = AuriMinimalBoxPrototype.Bind(hwp, styles);
                var boundCaption = AuriMinimalCaptionPrototype.Bind(hwp, styles);
#if DEBUG
                var figureSource = FigureSourcePrototype.Bind(hwp, binding.CaptionRoot + 1, profile.FigureSource);
#endif
                var box = plan.Summary.BoxOperations > 0 ? boundBox : null;
                var caption = plan.Summary.FigureOperations > 0 ? boundCaption : null;
#if DEBUG
                profile.Lists.Prototype("bullet").Bind(hwp);
                profile.Lists.Prototype("ordered").Bind(hwp);
                int start = PrepareInsertionTarget(hwp, profile);
                prepareTiming.Dispose();
                var insertTiming = RenderProfile.Measure("phase.insert");

                int? list = null;
                var continuations = new NativeListContinuations(document, plan);
                foreach (var operation in plan.Operations)
                {
                    if (verbose) Console.Error.WriteLine($"ir2hwp: {operation.Kind}/{operation.Label}");
                    using var operationTiming = RenderProfile.Measure("operation." + operation.Kind + "/" + operation.ParagraphStyle);
                    list = RenderOperation(hwp, operation, styles, box, caption, figureSource, list, continuations);
                }
                insertTiming.Dispose();
                var validateTiming = RenderProfile.Measure("phase.validate-flat");
                ClearNativeListAtCaret(hwp, styles.Resolve("body"));
                ApplyResolvedParagraphStyle(hwp, styles, styles.Resolve("body"));
                RemoveHyperlinksInRoots(hwp, start, ((XElement[])RangeRoots(hwp)).Length);
                VerifyStyles(hwp, plan, styles, start, continuations);
                VerifyCharacterMarks(hwp, plan, styles, start);
                VerifyBoxes(hwp, plan, styles, box, start);
                VerifyCaptions(hwp, plan, styles, caption, start);
                VerifyLists(hwp, plan, profile, start);
                RequireNoHyperlinks(((XElement[])RangeRoots(hwp)).Skip(start));
                // All clones are verified before removing the original prototypes.
                DeleteRangeParagraphs(hwp, binding.TemplateBegin, binding.TemplateEnd + 1);
                start -= binding.TemplateEnd + 1 - binding.TemplateBegin;
                validateTiming.Dispose();
                var saveFlatTiming = RenderProfile.Measure("phase.save-flat-reopen");
                if (!IndicatesSuccess(hwp.SaveAs(temporary, "HWP", "")))
                    throw new InvalidOperationException("Could not save rendered document.");
                CloseDocument(hwp); Open(hwp, temporary, visible);
                saveFlatTiming.Dispose();
                var verifyFlatTiming = RenderProfile.Measure("phase.verify-flat-reopened");
                VerifyText(hwp, plan, profile);
                VerifyStyles(hwp, plan, styles, start, continuations);
                VerifyCharacterMarks(hwp, plan, styles, start);
                VerifyBoxes(hwp, plan, styles, box, start, verifyPrototype: false);
                VerifyCaptions(hwp, plan, styles, caption, start, verifyPrototype: false);
                VerifyLists(hwp, plan, profile, start);
                XElement[] finalRoots = RangeRoots(hwp);
                RequireNoHyperlinks(finalRoots.Skip(start));
                XDocument finalDocument = RenderProfile.ReadDocument((object)hwp);
#else
                prepareTiming.Dispose();
                var composeTiming = RenderProfile.Measure("phase.compose-flat");
                var resources = CreateDirectPictureResources(hwp, document, plan, styles, boundBox, boundCaption, temporary, visible, out XElement[] pictures, out int? zOrderStart);
                var direct = DirectXmlFlatDocument.Compose(document, resources, binding, styles, plan, pictures, verbose, zOrderStart);
                int start = direct.Start;
                XDocument finalDocument = direct.Document;
                var continuations = direct.Lists.Continuations;
                if (direct.Lists.NeedsNativeMeasurement)
                {
                    using var listTiming = RenderProfile.Measure("compose.list-marker-measurement");
                    // Only native marker display is measured here. The expected
                    // document remains IR/template-derived, never an export of
                    // a potentially incorrect COM-generated intermediate.
                    ImportDirectMeasurementDocument(hwp, finalDocument);
                    direct.Lists.ResolveContinuations((object)hwp, finalDocument);
                }
                CanonicalizeDirectCharacterDefinitions(document, finalDocument);
                composeTiming.Dispose();
                var verifyFlatTiming = RenderProfile.Measure("phase.verify-composed-flat");
                XElement[] finalRoots = RangeRoots(finalDocument);
                IReadOnlyList<SavedParagraph> saved = ReadParagraphs(finalDocument);
                VerifyText(finalDocument, plan, profile);
                VerifyStyles(saved, plan, styles, start, continuations);
                VerifyCharacterMarks(saved, plan, styles, start);
                VerifyBoxes(finalDocument, plan, styles, box, start, verifyPrototype: false);
                VerifyCaptions(finalDocument, plan, styles, caption, start, verifyPrototype: false);
                VerifyLists(finalDocument, saved, plan, profile, start);
                RequireNoHyperlinks(finalRoots.Skip(start));
#endif
                TemplateRangeStructure.RequireOriginalStyleDefinitions(document, finalDocument, prefix);
                if (!TemplateRangeStructure.Equivalent(prefix, finalRoots.Take(prefix.Length).ToArray(), document, finalDocument))
                    throw new InvalidOperationException("Rendering changed static cover/header content: " +
                        TemplateRangeStructure.DescribeDifference(prefix, finalRoots.Take(prefix.Length).ToArray()));
                var expected = ExpectedParagraphs(plan, profile).ToArray();
                if (finalRoots.Length != prefix.Length + expected.Length + 1 || !IsSimpleParagraph(finalRoots[^1], ""))
                    throw new InvalidOperationException("Unexpected leftover template definitions or generated paragraph count.");
#if DEBUG
                IReadOnlyList<SavedParagraph> saved = ReadParagraphs(hwp);
#endif
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
                verifyFlatTiming.Dispose();
                var attachTiming = RenderProfile.Measure("phase.attach-native");
                var attached = boxes.Layout.Attach(finalDocument, plan, start);
                attached = nativeFigure.Layout.Attach(attached, plan, start);
                attached = tables.Layout.Attach(attached, plan, start, (table, target, prototype) =>
                    TableAutoWidths.Calculate(table, target, prototype, checked((int)Math.Round(tables.Layout.WidthMillimeters * 7200 / 25.4)), referenceTemplate.Layout));
                attached = objectSources.Layout.Attach(attached, plan, start);
                attached = preparedMetadata.Restore(headings.Layout.Attach(attached, plan, start, document =>
                {
                    boxes.Layout.AttachSpacing(document);
                    nativeFigure.Layout.AttachSpacing(document);
                    tables.Layout.AttachSpacing(document);
                }));
                attached = footnotes.Attach(attached, plan);
                TemplateHeadingNumbers.RequireResolved(attached);
                TemplateHeadingBlocks.RecalculateFigureNumbers(attached);
                tables.Layout.RecalculateNumbers(attached);
                TableWidthLimits.RequireFits(attached, tables.Layout.GeneratedTableInstances);
#if !DEBUG
                CanonicalizeDirectCharacterDefinitions(document, attached);
#endif
                var references = NativeCrossReferences.Prepare(attached, plan, referenceTemplate.Layout,
                    nativeFigure.Layout.GeneratedFigureInstances, tables.Layout.ReferencedTableInstances);
                attached = references.Document;
                footnotes.RecordLayout(attached);
                boxes.Layout.RecordLayout(attached);
                tables.Layout.RecordLayout(attached);
                attachTiming.Dispose();
                var nativeTiming = RenderProfile.Measure("phase.import-native-references");
                var tableStarts = NativeTablePagination.ResolveAnchors(attached, tables.Layout.GeneratedTableBodyInstances);
                ImportFigureDocument(hwp, attached, headings.Layout, boxes.Layout, footnotes, tables.Layout);
                references.Layout.Insert(hwp);
                if (references.Layout.Count > 0)
                {
                    // Replacing long markers with short native fields can leave
                    // Hancom's saved line layout stale. Import the completed
                    // controls to recalculate layout before measuring tables.
                    var completedReferences = RenderProfile.ReadDocument((object)hwp);
                    RequireFigureDocument(attached, completedReferences, headings.Layout, boxes: boxes.Layout,
                        footnotes: footnotes, references: references.Layout, tables: tables.Layout);
                    ImportFigureDocument(hwp, completedReferences, headings.Layout, boxes.Layout, footnotes, tables.Layout);
                    references.Layout.Verify(RenderProfile.ReadDocument((object)hwp));
                }
                // Measure the completed fields, whose displayed text can be much
                // shorter than the temporary reference markers in table cells.
                if (tableStarts.Count > 0)
                {
                    using var paginationTiming = RenderProfile.Measure("table.pagination");
                    var completed = RenderProfile.ReadDocument((object)hwp);
                    RequireFigureDocument(attached, completed, headings.Layout, boxes: boxes.Layout,
                        footnotes: footnotes, references: references.Layout, tables: tables.Layout);
                    var moved = NativeTablePagination.Reflow((object)hwp, completed,
                        tableStarts, corrected =>
                        {
                            ImportFigureDocument(hwp, corrected, headings.Layout, boxes.Layout, footnotes, tables.Layout);
                            references.Layout.Verify(RenderProfile.ReadDocument((object)hwp));
                        });
                    if (moved > 0)
                    {
                        var expectedRoots = AuriMinimalBoxPrototype.RootParagraphs(attached);
                        var completedRoots = AuriMinimalBoxPrototype.RootParagraphs(completed);
                        foreach (var anchor in tableStarts)
                        {
                            var i = anchor.RootParagraph;
                            expectedRoots[i].SetAttributeValue("PageBreak", (string?)completedRoots[i].Attribute("PageBreak"));
                            expectedRoots[i].SetAttributeValue("ColumnBreak", (string?)completedRoots[i].Attribute("ColumnBreak"));
                        }
                        if (verbose) Console.Error.WriteLine($"ir2hwp: moved {moved} orphaned table header(s) to the first body row's page.");
                    }
                }
                nativeTiming.Dispose();
                var saveNativeTiming = RenderProfile.Measure("phase.save-native-reopen");
                if (!IndicatesSuccess(hwp.SaveAs(temporary, "HWP", "")))
                    throw new InvalidOperationException("Could not save native figure captions.");
                CloseDocument(hwp); Open(hwp, temporary, visible);
                saveNativeTiming.Dispose();
                using var verifyNativeTiming = RenderProfile.Measure("phase.verify-native-reopened");
                var nativeSaved = RenderProfile.ReadDocument((object)hwp);
                RequireFigureDocument(attached, nativeSaved, headings.Layout, reportLayout: true, boxes: boxes.Layout, footnotes: footnotes, references: references.Layout, tables: tables.Layout);
                NativeTablePagination.VerifyReopened((object)hwp, tableStarts);
                return true;
            });
            if (HashFile(source) != hash) throw new InvalidOperationException("Source template changed.");
            PublishRenderedOutput(temporary, output);
            timing?.Complete();
            return new(output, IrContract.Version, true, true);
        }
        finally { if (File.Exists(temporary)) File.Delete(temporary); }
    }

    private static XDocument CreateDirectPictureResources(dynamic hwp, XDocument template,
        IrPreviewPlan plan, AuriPreviewStyleBindings styles, AuriMinimalBoxPrototype boundBox, AuriMinimalCaptionPrototype boundCaption,
        string temporary, bool visible, out XElement[] pictures, out int? zOrderStart)
    {
        using var timing = RenderProfile.Measure("compose.picture-resources");
        var figures = plan.Operations.Where(operation => operation.Kind == "figure").ToArray();
        if (figures.Length == 0)
        {
            pictures = [];
            var firstCode = plan.Operations.FirstOrDefault(operation => operation.Kind == "code");
            if (firstCode is null) { zOrderStart = null; return template; }
            // Read the native insertion-order origin rather than infer it from
            // static/template object ZOrders, which need not be sequential.
            boundBox.Insert(hwp, styles, firstCode.Lines, firstCode.SourceRuns);
            var codeResources = RenderProfile.ReadDocument((object)hwp);
            zOrderStart = (int?)AuriMinimalBoxPrototype.RootParagraphs(codeResources)[^2]
                .Descendants("TABLE").Single().Element("SHAPEOBJECT")!.Attribute("ZOrder")
                ?? throw new InvalidOperationException("Code resource has no native insertion order.");
            return codeResources;
        }
        var first = AuriMinimalBoxPrototype.RootParagraphs(template).Count - 1;
        Run(hwp, "MoveDocEnd");
        ApplyResolvedParagraphStyle(hwp, styles, styles.Resolve("figure"));
        foreach (var figure in figures)
        {
            object inserted = hwp.InsertPicture(figure.ImagePath!, true, 1, false, false, 0,
                figure.ImageWidthMillimeters!.Value, figure.ImageHeightMillimeters!.Value);
            if (!IndicatesSuccess(inserted))
                throw new InvalidOperationException($"Could not embed picture resource: {figure.ImagePath}");
            Run(hwp, "MoveParaEnd");
            Run(hwp, "BreakPara");
        }
        // Native HWP block insertion canonicalizes the body/master-page object
        // allocator, unlike InsertPicture or SaveAs alone. Prime it once with
        // the real caption prototype; discard this temporary paragraph in XML.
        boundCaption.Insert(hwp, styles, figures[0].FormattedLines![1]);
        XDocument resources;
        {
            // Saving materializes lazy image geometry and the native body /
            // master-page allocation order, including images already cached.
            // Both are needed to match native HWP clone insertion faithfully.
            using var materialize = RenderProfile.Measure("compose.materialize-picture-resources");
            if (!IndicatesSuccess(hwp.SaveAs(temporary, "HWP", "")))
                throw new InvalidOperationException("Could not materialize embedded picture resources.");
            CloseDocument(hwp); Open(hwp, temporary, visible);
            resources = RenderProfile.ReadDocument((object)hwp);
        }
        var roots = AuriMinimalBoxPrototype.RootParagraphs(resources);
        zOrderStart = (int?)roots[first].Descendants("PICTURE").Single().Element("SHAPEOBJECT")!.Attribute("ZOrder")
            ?? throw new InvalidOperationException("Picture resource has no native insertion order.");
        pictures = roots.Skip(first).SelectMany(paragraph => paragraph.Descendants("PICTURE"))
            .Select(picture => new XElement(picture)).ToArray();
        if (roots.Count != first + figures.Length + 2 || pictures.Length != figures.Length)
            throw new InvalidOperationException("Picture resource insertion changed unexpected document structure.");
        if (pictures.Any(picture => picture.Element("IMAGEDIM") is not { } dimensions ||
            (int?)dimensions.Attribute("Width") is null or <= 0 || (int?)dimensions.Attribute("Height") is null or <= 0))
            throw new InvalidOperationException("Native picture dimensions remain unresolved after materialization.");
        return resources;
    }

    private static void CanonicalizeDirectCharacterDefinitions(XDocument template, XDocument generated)
    {
        var known = template.Descendants("CHARSHAPE").Select(shape => (string?)shape.Attribute("Id")).ToHashSet();
        foreach (var shape in generated.Descendants("CHARSHAPE").Where(shape => !known.Contains((string?)shape.Attribute("Id"))))
            DirectXmlParagraphs.OrderMarks(shape);
    }

    private static void ImportDirectMeasurementDocument(dynamic hwp, XDocument document)
    {
        _ = hwp.Clear(1);
        object imported = hwp.SetTextFile("<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>" +
            document.ToString(SaveOptions.DisableFormatting), "HWPML2X", "");
        if (imported is not int status || status != 1)
            throw new InvalidOperationException("Could not import list marker measurement document.");
    }

    private static void ImportFigureDocument(dynamic hwp, XDocument document, TemplateHeadingBlocks? headings = null, TemplateBoxParagraphs? boxes = null, NativeFootnotes? footnotes = null, TemplateTables? tables = null)
    {
        _ = hwp.Clear(1);
        object imported = hwp.SetTextFile("<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>" + document.ToString(SaveOptions.DisableFormatting), "HWPML2X", "");
        if (imported is not int status || status != 1) throw new InvalidOperationException("Could not import native figure caption structure.");
        RequireFigureDocument(document, RenderProfile.ReadDocument((object)hwp), headings, boxes: boxes, footnotes: footnotes, tables: tables);
    }

    private static void RequireFigureDocument(XDocument expected, XDocument actual, TemplateHeadingBlocks? headings = null, bool reportLayout = false, TemplateBoxParagraphs? boxes = null, NativeFootnotes? footnotes = null, NativeCrossReferences? references = null, TemplateTables? tables = null)
    {
        expected = new XDocument(expected);
        actual = new XDocument(actual);
        headings?.NormalizeTitleLayout(expected, actual, reportLayout);
        boxes?.NormalizeLayout(expected, actual);
        tables?.NormalizeLayout(expected, actual);
        footnotes?.NormalizeNumbers(expected, actual);
        references?.NormalizeExpected(expected, actual);
        // Hancom adds identity scale/rotation pairs during HWPML import.
        // Ignore only mathematically neutral matrices in comparison copies.
#if DEBUG
        expected = NormalizeFigureMatrices(expected);
        actual = NormalizeFigureMatrices(actual);
#else
        NormalizeFigureMatricesInPlace(expected);
        NormalizeFigureMatricesInPlace(actual);
#endif
        var before = AuriMinimalBoxPrototype.RootParagraphs(expected);
        var after = AuriMinimalBoxPrototype.RootParagraphs(actual);
        TemplateRangeStructure.RequireOriginalStyleDefinitions(expected, actual, before);
        if (!TemplateRangeStructure.Equivalent(before, after, expected, actual))
            throw new InvalidOperationException("Native caption import changed document structure: " + TemplateRangeStructure.DescribeDifference(before, after, expected, actual));
        foreach (var image in before.SelectMany(p => p.Descendants("IMAGE")))
        {
            var id = (string?)image.Attribute("BinItem") ?? throw new InvalidOperationException("Missing embedded image reference.");
            var original = expected.Descendants("BINDATA").Single(e => (string?)e.Attribute("Id") == id);
            var saved = actual.Descendants("BINDATA").SingleOrDefault(e => (string?)e.Attribute("Id") == id);
            if (saved is null || !Convert.FromBase64String(original.Value).SequenceEqual(Convert.FromBase64String(saved.Value)))
                throw new InvalidOperationException($"Native caption import changed embedded image {id}.");
        }
    }

    internal static XDocument NormalizeFigureMatrices(XDocument document)
    {
        var copy = new XDocument(document);
        NormalizeFigureMatricesInPlace(copy);
        return copy;
    }

    private static void NormalizeFigureMatricesInPlace(XDocument copy)
    {
        // Group children retain original dimensions and transformation matrices;
        // Hancom may omit their derived current dimensions after saving.
        foreach (var component in copy.Descendants("SHAPECOMPONENT").Where(e => (int?)e.Attribute("GroupLevel") > 0))
        {
            component.Attribute("CurWidth")?.Remove();
            component.Attribute("CurHeight")?.Remove();
        }
        // Text-box layout width is a calculated cache, not its frame geometry.
        foreach (var text in copy.Descendants("DRAWTEXT")) text.Attribute("LastWidth")?.Remove();
        foreach (var matrix in copy.Descendants("RENDERINGINFO").Elements()
                     .Where(e => e.Name.LocalName is "SCAMATRIX" or "ROTMATRIX").ToArray())
        {
            var identity = new[] { 1m, 0m, 0m, 0m, 1m, 0m };
            if (matrix.Attributes().Count() == 6 && Enumerable.Range(1, 6).All(i =>
                decimal.TryParse((string?)matrix.Attribute("E" + i), System.Globalization.NumberStyles.Float,
                    System.Globalization.CultureInfo.InvariantCulture, out var value) && value == identity[i - 1])) matrix.Remove();
        }
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

    private static XElement[] RangeRoots(dynamic hwp)
    {
        var document = RenderProfile.ReadDocument((object)hwp);
        return RangeRoots(document);
    }

    private static XElement[] RangeRoots(XDocument document)
    {
        var sections = document.Descendants().Where(e => e.Name.LocalName == "SECTION").ToArray();
        if (sections.Length != 1) throw new InvalidOperationException("Tagged rendering requires one section.");
        return sections[0].Elements().Where(e => e.Name.LocalName == "P").ToArray();
    }

    private static void SelectRangeParagraphs(dynamic hwp, int start, int end)
    {
        XElement[] roots = RangeRoots(hwp);
        SelectRangeParagraphs(hwp, start, end, roots);
    }

    private static void SelectRangeParagraphs(dynamic hwp, int start, int end, XElement[] roots)
    {
        if (start < 0 || end <= start || end >= roots.Length)
            throw new InvalidOperationException("Invalid range or missing successor.");
        if (!(bool)hwp.SetPos(0, start, 0)) throw new InvalidOperationException("Cannot resolve range beginning.");
        Run(hwp, "MoveParaBegin");
        for (var i = start; i < end; i++) Run(hwp, "MoveSelNextParaBegin");
    }

    private static void DeleteRangeParagraphs(dynamic hwp, int start, int end)
    {
        XElement[] before = RangeRoots(hwp);
        var expected = before.Take(start).Concat(before.Skip(end)).Select(p => p.Value).ToArray();
        RequireRangeText(hwp, before.Select(p => p.Value).ToArray(), "immediately before deletion");
        SelectRangeParagraphs(hwp, start, end);
        Run(hwp, "Delete");
        RequireRangeText(hwp, expected, "immediately after deletion");
    }

    private static void DeleteRangeParagraphs(dynamic hwp, int start, int end, XElement[] before)
    {
        // Reuse the already verified, unchanged snapshot for range/successor
        // checks, then verify actual deletion with a fresh native export.
        var expected = before.Take(start).Concat(before.Skip(end)).Select(p => p.Value).ToArray();
        SelectRangeParagraphs(hwp, start, end, before);
        Run(hwp, "Delete");
        RequireRangeText(hwp, expected, "immediately after deletion");
    }

    private static void RequireRangeText(dynamic hwp, string[] expected, string stage)
    {
        XElement[] roots = RangeRoots(hwp);
        RequireRangeText(roots, expected, stage);
    }

    private static void RequireRangeText(XElement[] roots, string[] expected, string stage)
    {
        var actual = roots.Select(p => p.Value).ToArray();
        if (!actual.SequenceEqual(expected, StringComparer.Ordinal))
            throw new InvalidOperationException($"{stage}: expected {System.Text.Json.JsonSerializer.Serialize(expected)}; actual {System.Text.Json.JsonSerializer.Serialize(actual)}");
    }

    private static void VerifyTaggedLineBreaks(IReadOnlyList<XElement> roots, IrPreviewPlan plan)
    {
        var index = 0;
        foreach (var operation in plan.Operations)
        {
            if (operation.Kind == "text" && roots[index].Descendants().Count(e => e.Name.LocalName == "LINEBREAK") != operation.Lines.Count - 1)
                throw new InvalidOperationException("IR line breaks did not remain within their paragraph.");
            index += operation.Kind == "figure" ? (operation.Lines[2].Length > 0 ? 3 : 2) : 1;
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
            index += operation.Kind == "figure" ? (operation.Lines[2].Length > 0 ? 3 : 2) : 1;
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
