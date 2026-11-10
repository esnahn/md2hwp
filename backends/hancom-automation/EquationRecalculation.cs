// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record EquationRecalculationOptions(string SourcePath, string OutputPath)
{
    internal const string Usage = """
        usage: hwpeqrecalc <source.hwp|hwpx> [[--output] <output.hwp|hwpx>]
               hwpeqrecalc --input <source> [--output <output>]
               hwpeqrecalc --version
        Opens the document in Hancom, lays out every equation again with its own
        script, size and font, and saves the result. Hancom keeps stored equation
        sizes when it opens a file; use this after another program wrote them.
        Default output: source.recalc.hwp (or .hwpx) beside the source, which is
        never modified. The output format follows the output extension.
        Requires Windows x64, Hancom HWP, .NET 10 x64 and the registered Hancom security module.
        """ + LicenseNotice.Text;

    internal static EquationRecalculationOptions Parse(string[] args)
    {
        string? source = null, output = null;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        for (var index = 0; index < args.Length; index++)
        {
            var value = args[index];
            if (value is "--input" or "--output")
            {
                if (!seen.Add(value)) throw new ArgumentException($"Duplicate argument: {value}");
                if (++index >= args.Length || args[index].StartsWith("-", StringComparison.Ordinal))
                    throw new ArgumentException($"Missing value for {value}.\n{Usage}");
                if (value == "--input")
                {
                    if (source is not null) throw new ArgumentException("Duplicate input argument.");
                    source = args[index];
                }
                else
                {
                    if (output is not null) throw new ArgumentException("Duplicate output argument.");
                    output = args[index];
                }
            }
            else
            {
                if (value.StartsWith("-", StringComparison.Ordinal)) throw new ArgumentException(Usage);
                if (source is null) source = value;
                else if (output is null) output = value;
                else throw new ArgumentException(Usage);
            }
        }
        if (string.IsNullOrWhiteSpace(source)) throw new ArgumentException(Usage);
        source = Path.GetFullPath(source);
        _ = NativeDocumentFormat.FromPath(source);
        output = Path.GetFullPath(output ?? Path.Combine(Path.GetDirectoryName(source)!,
            Path.GetFileNameWithoutExtension(source) + ".recalc" + Path.GetExtension(source).ToLowerInvariant()));
        _ = NativeDocumentFormat.FromPath(output);
        return new(source, output);
    }

    internal void ValidatePaths()
    {
        if (!File.Exists(SourcePath)) throw new FileNotFoundException($"Missing document: {SourcePath}", SourcePath);
        if (!Directory.Exists(Path.GetDirectoryName(OutputPath)))
            throw new DirectoryNotFoundException($"Missing output directory: {Path.GetDirectoryName(OutputPath)}");
        if (Directory.Exists(OutputPath)) throw new IOException($"Output is a directory: {OutputPath}");
        var source = new FileInfo(SourcePath).ResolveLinkTarget(true)?.FullName ?? SourcePath;
        var output = File.Exists(OutputPath)
            ? new FileInfo(OutputPath).ResolveLinkTarget(true)?.FullName ?? OutputPath : OutputPath;
        if (source.Equals(output, StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Output must not replace the source document.");
    }
}

internal sealed record EquationRecalculationResult(string Output, int Equations, int Resized, int TablesResized, int ImagesReencoded,
    IReadOnlyList<string> Warnings);

// One equation as Hancom exports it, plus the font only its control reports.
internal sealed record EquationState(string Script, string BaseUnit, string Font, int Width, int Height, int Baseline);

internal static partial class HancomPreviewWriter
{
    internal static EquationRecalculationResult RecalculateEquations(EquationRecalculationOptions options)
    {
        options.ValidatePaths();
        EnsureInteractiveContext();
        var module = SecurityModuleRegistration.ReadAndValidate(AppContext.BaseDirectory);
        var sourceHash = HashFile(options.SourcePath);
        var temporary = NativeDocumentFormat.TemporaryPath(options.OutputPath, "hwpeqrecalc");
        var hwpx = NativeDocumentFormat.FromPath(options.OutputPath) == "HWPX";
        var converting = NativeDocumentFormat.FromPath(options.SourcePath) != NativeDocumentFormat.FromPath(options.OutputPath);
        string? snapshot = null;
        var warnings = new List<string>();
        try
        {
            snapshot = SharedFile.Snapshot(options.SourcePath);
            var (count, resized, tables, images) = WithHwp(module, hwp =>
            {
                hwp.XHwpWindows.Item(0).Visible = false;
                Open(hwp, snapshot!, visible: false);
                var original = HwpMarkup.ReadDocument((object)hwp);
                List<EquationState> before = ReadEquations(hwp, original);
                foreach (var control in EquationControls((object)hwp))
                {
                    // Setting a property, even to its current value, makes
                    // Hancom lay the equation out again.
                    var properties = control.Properties;
                    properties.SetItem("EqFontName", (string)properties.Item("EqFontName"));
                    control.Properties = properties;
                }
                var recalculated = HwpMarkup.ReadDocument((object)hwp);
                List<EquationState> after = ReadEquations(hwp, recalculated);
                RequireSameEquations(before, after);
                // Other changes are reported, not fatal: the output is still written.
                if (DescribeNonEquationChange(original, recalculated, hwpx: false) is { } relayout)
                    warnings.Add("Recalculation changed more than equation geometry near: " + relayout);

                if (!IndicatesSuccess(hwp.SaveAs(temporary, NativeDocumentFormat.FromPath(temporary), "")))
                    throw new InvalidOperationException("Could not save the recalculated document.");
                CloseDocument(hwp);
                Open(hwp, temporary, visible: false);
                var reopened = HwpMarkup.ReadDocument((object)hwp);
                List<EquationState> saved = ReadEquations(hwp, reopened);
                if (!saved.SequenceEqual(after))
                    throw new InvalidOperationException("Recalculated equations changed after save and reopen.");
                if (DescribeNonEquationChange(recalculated, reopened, hwpx, exact: true) is { } saving)
                    warnings.Add(converting
                        ? $"Saving as {NativeDocumentFormat.FromPath(options.OutputPath)} changed content outside equations; " +
                          "use the source format for the output to avoid this. Near: " + saving
                        : "Save and reopen changed content outside equations near: " + saving);
                CloseDocument(hwp);
                return (after.Count, before.Zip(after).Count(pair => pair.First != pair.Second),
                    TableHeights(original).Zip(TableHeights(recalculated)).Count(pair => pair.First != pair.Second),
                    ImageData(recalculated).Zip(ImageData(reopened)).Count(pair => pair.First != pair.Second));
            });
            if (!HashFile(options.SourcePath).Equals(sourceHash, StringComparison.Ordinal))
                throw new InvalidOperationException("Source document changed during recalculation; output was not replaced.");
            File.Move(temporary, options.OutputPath, overwrite: true);
            return new(options.OutputPath, count, resized, tables, images, warnings);
        }
        finally
        {
            if (File.Exists(temporary)) File.Delete(temporary);
            if (snapshot is not null) SharedFile.DeleteSnapshot(snapshot);
        }
    }

    private static IEnumerable<string?> TableHeights(XDocument document) => document.Descendants("TABLE")
        .Select(table => (string?)table.Element("SHAPEOBJECT")?.Element("SIZE")?.Attribute("Height"));

    private static IEnumerable<string> ImageData(XDocument document) => document.Descendants("BINDATA")
        .Select(data => data.Value);

    private static IReadOnlyList<dynamic> EquationControls(object application)
    {
        dynamic hwp = application;
        var controls = new List<dynamic>();
        for (dynamic? control = hwp.HeadCtrl; control is not null; control = control.Next)
            if ((string)control.CtrlID == "eqed") controls.Add(control);
        return controls;
    }

    // The control chain and the export list equations in the same order,
    // including notes, cells, text boxes and headers; require that pairing.
    private static List<EquationState> ReadEquations(object application, XDocument document)
    {
        dynamic hwp = application;
        var equations = document.Descendants("EQUATION").ToArray();
        var controls = EquationControls(application);
        if (controls.Count != equations.Length)
            throw new InvalidOperationException("Native equation controls do not match the exported equations.");
        var result = new List<EquationState>();
        for (var i = 0; i < equations.Length; i++)
        {
            var equation = equations[i];
            var script = (string?)equation.Element("SCRIPT") ?? "";
            if ((string?)controls[i].Properties.Item("String") != script)
                throw new InvalidOperationException("Native equation controls are not in exported document order.");
            var size = equation.Element("SHAPEOBJECT")?.Element("SIZE")
                ?? throw new InvalidOperationException("An equation has no stored size.");
            result.Add(new(script, (string?)equation.Attribute("BaseUnit") ?? "",
                (string)controls[i].Properties.Item("EqFontName"),
                (int?)size.Attribute("Width") ?? 0, (int?)size.Attribute("Height") ?? 0,
                (int?)equation.Attribute("BaseLine") ?? 0));
        }
        return result;
    }

    private static void RequireSameEquations(IReadOnlyList<EquationState> before, IReadOnlyList<EquationState> after)
    {
        if (before.Count != after.Count ||
            before.Zip(after).Any(pair => pair.First.Script != pair.Second.Script ||
                pair.First.BaseUnit != pair.Second.BaseUnit || pair.First.Font != pair.Second.Font))
            throw new InvalidOperationException("Recalculation changed an equation's script, size or font.");
    }

    // Everything but equation width, height and baseline should stay the same.
    // With exact, the stored geometry must also match (save/reopen check).
    // Returns the first difference ("expected | actual" excerpts), or null.
    private static string? DescribeNonEquationChange(XDocument expected, XDocument actual, bool hwpx, bool exact = false)
    {
        string Canonical(XDocument document)
        {
            var copy = new XDocument(document);
            if (!exact)
            {
                foreach (var equation in copy.Descendants("EQUATION"))
                {
                    equation.Attribute("BaseLine")?.Remove();
                    var size = equation.Element("SHAPEOBJECT")?.Element("SIZE");
                    size?.Attribute("Width")?.Remove();
                    size?.Attribute("Height")?.Remove();
                }
                // Re-layout also refreshes table and cell heights that the
                // writing program stored; they are derived from content.
                foreach (var table in copy.Descendants("TABLE"))
                    table.Element("SHAPEOBJECT")?.Element("SIZE")?.Attribute("Height")?.Remove();
                foreach (var cell in copy.Descendants("CELL")) cell.Attribute("Height")?.Remove();
            }
            // Hancom may re-encode embedded images when it saves; keep the
            // resources and their references, not the encoded bytes.
            foreach (var data in copy.Descendants("BINDATA"))
            {
                data.Attribute("Size")?.Remove();
                data.Value = "";
            }
            NormalizeFigureMatricesInPlace(copy);
            if (hwpx)
            {
                NormalizeHwpxMatrixPrecisionInPlace(copy);
                NormalizeHwpxImageIdentitiesInPlace(copy);
                // HWPX keeps no default script header when there is no script.
                foreach (var code in copy.Descendants("SCRIPTCODE").ToArray())
                    if (string.IsNullOrWhiteSpace((string?)code.Element("SCRIPTSOURCE"))) code.Remove();
            }
            copy.Root?.Element("HEAD")?.Element("DOCSUMMARY")?.Remove();
            return copy.ToString(SaveOptions.DisableFormatting);
        }
        var left = Canonical(expected);
        var right = Canonical(actual);
        if (left == right) return null;
        var index = 0;
        while (index < Math.Min(left.Length, right.Length) && left[index] == right[index]) index++;
        var start = Math.Max(0, index - 80);
        return left.Substring(start, Math.Min(200, left.Length - start)) + " | " +
            right.Substring(start, Math.Min(200, right.Length - start));
    }
}
