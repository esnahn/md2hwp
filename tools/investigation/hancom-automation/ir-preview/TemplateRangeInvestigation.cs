using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TemplateRangeResult(string Authored, string Output,
    string SourceSha256, int CopiedParagraphs, bool AuthoredReopened,
    bool OutputReopened, bool SurroundingTextVerified, bool SourceUnchanged);

internal static partial class HancomPreviewWriter
{
    // Fixture-specific, text-only experiment. This is not a template renderer.
    public static TemplateRangeResult InvestigateRanges(string templatePath,
        string authoredPath, string outputPath, string repositoryRoot, bool visible)
    {
        var template = ValidateTemplate(templatePath);
        var authored = ValidateNewHwpPath(authoredPath, "Authored fixture");
        var output = ValidateNewHwpPath(outputPath, "Range result");
        if (string.Equals(authored, output, StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Authored fixture and output must differ.");
        var hash = HashFile(template);
        var fixture = Path.Combine(repositoryRoot, "tests", "fixtures", "templates", "minimal.hwp");
        if (hash != HashFile(fixture))
            throw new ArgumentException("Range investigation requires the exact tracked minimal.hwp.");
        EnsureInteractiveContext();
        EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(repositoryRoot);
        var authoredTemp = Path.Combine(Path.GetDirectoryName(authored)!, $".range-author-{Guid.NewGuid():N}.hwp");
        var outputTemp = Path.Combine(Path.GetDirectoryName(output)!, $".range-result-{Guid.NewGuid():N}.hwp");
        bool authoredPublished = false, outputPublished = false;
        try
        {
            File.Copy(template, authoredTemp, overwrite: false);
            WithHwp(module, hwp =>
            {
                Open(hwp, authoredTemp, visible);
                XElement[] original = RangeRoots(hwp);
                var originalText = original.Select(p => p.Value).ToArray();
                if (originalText.Any(t => t.Contains("{{md2hwp:", StringComparison.Ordinal)))
                    throw new InvalidOperationException("Source already contains reserved declarations.");
                Run(hwp, "MoveDocEnd");
                Run(hwp, "BreakPara");
                string[] appended = ["{{md2hwp:content}}", "", "RANGE-GUARD: 보존할 뒷부분",
                    "{{md2hwp:begin:samples}}", "{{md2hwp:contract:experimental-1}}",
                    "{{md2hwp:body}}", "{{md2hwp:heading.1}}", "{{md2hwp:end:samples}}", ""];
                for (var i = 0; i < appended.Length; i++)
                {
                    if (appended[i].Length > 0) InsertText(hwp, appended[i]);
                    if (i + 1 < appended.Length) Run(hwp, "BreakPara");
                }
                var authoredText = originalText.Concat(appended).ToArray();
                RequireRangeText(hwp, authoredText, "authoring");
                Run(hwp, "FileSave");
                CloseDocument(hwp);
                Open(hwp, authoredTemp, visible);
                RequireRangeText(hwp, authoredText, "authored reopen");
                CloseDocument(hwp);
                File.Copy(authoredTemp, outputTemp, overwrite: false);
                Open(hwp, outputTemp, visible);

                XElement[] roots = RangeRoots(hwp);
                var plan = TemplateDeclarations.Parse(roots.Select(p => p.Value).ToArray());
                var start = plan.Styles["body"];
                var end = plan.Styles["heading.1"] + 1;
                var cloneText = roots[start..end].Select(p => p.Value).ToArray();
                var cloneStyles = roots[start..end].Select(p => (string?)p.Attribute("Style")).ToArray();
                SelectRangeParagraphs(hwp, start, end);
                var nativeBlock = (string)hwp.GetTextFile("HWP", "saveblock");
                Run(hwp, "Cancel");
                if (string.IsNullOrWhiteSpace(nativeBlock))
                    throw new InvalidOperationException("Empty selected native paragraph block.");

                DeleteRangeParagraphs(hwp, plan.SamplesBegin, plan.SamplesEnd + 1);
                roots = RangeRoots(hwp);
                // Resolve again after deletion. Never reuse earlier COM positions.
                var targets = roots.Select((p, i) => (p, i))
                    .Where(x => IsSimpleParagraph(x.p, "{{md2hwp:content}}")).ToArray();
                if (targets.Length != 1 || targets[0].i + 1 >= roots.Length ||
                    !IsSimpleParagraph(roots[targets[0].i + 1], ""))
                    throw new InvalidOperationException("Expected unique content target with empty successor.");
                var target = targets[0].i;
                DeleteRangeParagraphs(hwp, target, target + 1);
                XElement[] insertionRoots = RangeRoots(hwp);
                var beforeInsert = insertionRoots.Select(p => p.Value).ToArray();
                if (!(bool)hwp.SetPos(0, target, 0)) throw new InvalidOperationException("Cannot resolve insertion position.");
                object? inserted = hwp.SetTextFile(nativeBlock, "HWP", "insertfile");
                if (inserted is bool ok && !ok) throw new InvalidOperationException("Native insertion rejected.");
                var expected = beforeInsert.Take(target).Concat(cloneText).Concat(beforeInsert.Skip(target)).ToArray();
                RequireRangeText(hwp, expected, "native range insertion");
                RequireCloneStyles(hwp, target, cloneStyles);
                Run(hwp, "FileSave");
                CloseDocument(hwp);
                Open(hwp, outputTemp, visible);
                RequireRangeText(hwp, expected, "result reopen");
                RequireCloneStyles(hwp, target, cloneStyles);
                XElement[] finalRoots = RangeRoots(hwp);
                if (!finalRoots.Take(original.Length).Select(p => p.Value).SequenceEqual(originalText))
                    throw new InvalidOperationException("Original text changed.");
                if (!TemplateRangeStructure.Equivalent(original, finalRoots.Take(original.Length).ToArray()))
                    throw new InvalidOperationException("Original paragraph structure or relative ZOrder changed.");
                foreach (var name in new[] { "TABLE", "PICTURE", "AUTONUM" })
                    if (original.SelectMany(p => p.Descendants()).Count(e => e.Name.LocalName == name) !=
                        finalRoots.SelectMany(p => p.Descendants()).Count(e => e.Name.LocalName == name))
                        throw new InvalidOperationException($"Original {name} count changed.");
                return true;
            });
            if (HashFile(template) != hash) throw new InvalidOperationException("Source template changed.");
            File.Move(authoredTemp, authored);
            authoredPublished = true;
            File.Move(outputTemp, output);
            outputPublished = true;
            return new(authored, output, hash, 2, true, true, true, true);
        }
        catch
        {
            if (outputPublished) File.Delete(output);
            if (authoredPublished) File.Delete(authored);
            throw;
        }
        finally
        {
            if (File.Exists(outputTemp)) File.Delete(outputTemp);
            if (File.Exists(authoredTemp)) File.Delete(authoredTemp);
        }
    }

    private static XElement[] RangeRoots(dynamic hwp)
    {
        var document = XDocument.Parse((string)hwp.GetTextFile("HWPML2X", ""));
        var sections = document.Descendants().Where(e => e.Name.LocalName == "SECTION").ToArray();
        if (sections.Length != 1) throw new InvalidOperationException("Range experiment requires one section.");
        return sections[0].Elements().Where(e => e.Name.LocalName == "P").ToArray();
    }

    private static void SelectRangeParagraphs(dynamic hwp, int start, int end)
    {
        XElement[] roots = RangeRoots(hwp);
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

    private static void RequireRangeText(dynamic hwp, string[] expected, string stage)
    {
        XElement[] roots = RangeRoots(hwp);
        var actual = roots.Select(p => p.Value).ToArray();
        if (!actual.SequenceEqual(expected, StringComparer.Ordinal))
            throw new InvalidOperationException($"{stage}: expected {System.Text.Json.JsonSerializer.Serialize(expected)}; actual {System.Text.Json.JsonSerializer.Serialize(actual)}");
    }

    private static void RequireCloneStyles(dynamic hwp, int start, string?[] expected)
    {
        XElement[] roots = RangeRoots(hwp);
        if (!roots.Skip(start).Take(expected.Length).Select(p => (string?)p.Attribute("Style")).SequenceEqual(expected))
            throw new InvalidOperationException("Cloned paragraph styles changed.");
    }
}
