using System.ComponentModel;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Text;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

/// <summary>
/// Measures the manuscript with the prototype's actual fonts, then fits the
/// preferred widths into the declared table width. Native logical units are
/// scaled to HWP's 1/7200 inch units, without a desktop .NET dependency.
/// </summary>
internal static class TableAutoWidths
{
    internal static int[] Calculate(PreviewTable table, XDocument document, XElement tablePrototype, int widthHwpUnits, TemplateCrossReferences? references = null)
    {
        if (widthHwpUnits <= 0 || table.Header.Count == 0)
            throw new InvalidDataException("Table width and column count must be positive.");
        var prototype = tablePrototype.DescendantsAndSelf("TABLE").Single();
        var rows = prototype.Elements("ROW").ToArray();
        if (rows.Length != 3 || rows.Take(2).Any(row => row.Elements("CELL").Count() != 2) || rows[2].Elements("CELL").Count() != 1)
            throw new InvalidDataException("Table width calculation requires two header/content cells and one merged source sample.");
        var header = CellFormat.Read(rows[0].Element("CELL")!, prototype, document);
        var body = CellFormat.Read(rows[1].Element("CELL")!, prototype, document);
        using var measurement = new NativeFontMeasurement(new XDocument(document), references);
        var minimum = new double[table.Header.Count];
        var preferred = new double[minimum.Length];
        for (var column = 0; column < minimum.Length; column++)
        {
            Add(table.Header[column], header);
            foreach (var row in table.Rows) Add(row[column], body);
            void Add(PreviewInlineContent content, CellFormat format)
            {
                var bounds = measurement.Measure(content, format);
                // Hancom and GDI round/hint outlines differently. A small
                // allowance avoids wrapping dates at an exact measured edge.
                minimum[column] = Math.Max(minimum[column], bounds.Minimum * 1.01 + format.HorizontalMargins);
                preferred[column] = Math.Max(preferred[column], bounds.Preferred * 1.01 + format.HorizontalMargins);
            }
            minimum[column] = Math.Max(1, minimum[column]);
            preferred[column] = Math.Max(minimum[column], preferred[column]);
        }
        return Distribute(minimum, preferred, widthHwpUnits);
    }

    internal static double MeasurePlainText(XDocument document, XElement characterShape, string text,
        bool strong = false, bool emphasis = false)
    {
        using var measurement = new NativeFontMeasurement(document, null);
        return measurement.MeasureText(text, characterShape, strong, emphasis);
    }

    internal static int[] Distribute(IReadOnlyList<double> minimum, IReadOnlyList<double> preferred, int total)
    {
        if (minimum.Count == 0 || minimum.Count != preferred.Count || total <= 0 ||
            minimum.Any(v => !double.IsFinite(v) || v < 0) ||
            preferred.Any(v => !double.IsFinite(v) || v < 0))
            throw new InvalidDataException("Invalid measured table widths.");
        var floor = minimum.Select(v => checked((int)Math.Ceiling(Math.Max(1, v)))).ToArray();
        if (floor.Sum(v => (long)v) > total)
            throw new InvalidDataException("The declared table width is smaller than the cells' minimum text widths and margins. Increase table.width-mm, reduce the cell margins/font size, or allow word wrapping in the prototype.");
        var remainder = total - floor.Sum();
        var weights = preferred.Select((v, i) => Math.Max(0, v - floor[i])).ToArray();
        // When everything already fits, distribute spare space in proportion to
        // the measured preferred widths. A short ID column remains short.
        if (weights.Sum() <= remainder)
            weights = preferred.Select((v, i) => Math.Max(floor[i], v)).ToArray();
        var weightSum = weights.Sum();
        if (weightSum <= 0) weights = Enumerable.Repeat(1.0, floor.Length).ToArray();
        weightSum = weights.Sum();
        var shares = weights.Select(weight => remainder * weight / weightSum).ToArray();
        var result = floor.Select((v, i) => v + (int)Math.Floor(shares[i])).ToArray();
        var missing = total - result.Sum();
        foreach (var index in Enumerable.Range(0, result.Length)
            .OrderByDescending(i => shares[i] - Math.Floor(shares[i])).ThenBy(i => i).Take(missing))
            result[index]++;
        return result;
    }

    private sealed record CellFormat(XElement CharacterShape, bool KeepLatinWords, bool KeepOtherWords, double HorizontalMargins)
    {
        internal static CellFormat Read(XElement cell, XElement table, XDocument document)
        {
            var paragraph = cell.Element("PARALIST")?.Elements("P").Single()
                ?? throw new InvalidDataException("Table sample cell must contain one paragraph.");
            var run = paragraph.Elements("TEXT").FirstOrDefault(text => text.Elements("CHAR").Any(c => c.Value.Length > 0))
                ?? throw new InvalidDataException("Table sample slot has no character formatting.");
            var shape = document.Descendants("CHARSHAPE").Single(e =>
                (string?)e.Attribute("Id") == (string?)run.Attribute("CharShape"));
            var paragraphShape = document.Descendants("PARASHAPE").Single(e =>
                (string?)e.Attribute("Id") == (string?)paragraph.Attribute("ParaShape"));
            var margin = (string?)cell.Attribute("HasMargin") == "true"
                ? cell.Element("CELLMARGIN") : table.Element("INSIDEMARGIN");
            var paragraphMargin = paragraphShape.Element("PARAMARGIN");
            static int Nonnegative(XElement? e, string name) => Math.Max(0, (int?)e?.Attribute(name) ?? 0);
            // Legacy HWPML paragraph margins use URC (1/14400 inch),
            // whereas table/cell margins and widths use HWPUNIT (1/7200 inch).
            var horizontal = Nonnegative(margin, "Left") + Nonnegative(margin, "Right") +
                (Nonnegative(paragraphMargin, "Left") + Nonnegative(paragraphMargin, "Right") +
                Math.Abs((double)((int?)paragraphMargin?.Attribute("Indent") ?? 0))) / 2.0;
            return new(shape,
                (string?)paragraphShape.Attribute("BreakLatinWord") == "KeepWord",
                (string?)paragraphShape.Attribute("BreakNonLatinWord") == "false", horizontal);
        }
    }

    private sealed class NativeFontMeasurement : IDisposable
    {
        private readonly XDocument document;
        private readonly TemplateCrossReferences? references;
        private readonly nint device;
        private readonly Dictionary<FontKey, nint> fonts = [];
        private readonly Dictionary<(FontKey Font, string Text), double> widths = [];
        private readonly nint originalFont;
        private const int LogicalScale = 4;

        internal NativeFontMeasurement(XDocument document, TemplateCrossReferences? references)
        {
            this.document = document;
            this.references = references;
            device = Native.CreateCompatibleDC(0);
            if (device == 0) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not create a font measurement context.");
            originalFont = Native.GetCurrentObject(device, 6); // OBJ_FONT
            if (originalFont == 0)
            {
                _ = Native.DeleteDC(device);
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not read the measurement context's original font.");
            }
        }

        internal (double Minimum, double Preferred) Measure(PreviewInlineContent content, CellFormat format)
        {
            var minimum = 0.0;
            var preferred = 0.0;
            foreach (var line in content.Lines)
            {
                var lineWidth = 0.0;
                var wordWidth = 0.0;
                var previousGap = 0.0;
                var wordGap = 0.0;
                foreach (var run in line.Runs)
                {
                    if (run.Footnote is not null || run.CrossReference is not null)
                    {
                        // Number fields resolve later in Hancom. Reserve measured
                        // display text; never measure the internal GUID marker.
                        var reserved = ReserveNative(run, format);
                        lineWidth += previousGap + reserved.Width;
                        minimum = Math.Max(minimum, wordWidth + wordGap + reserved.Width);
                        previousGap = reserved.LastGap;
                        wordWidth = 0; wordGap = 0;
                        continue;
                    }
                    var elements = StringInfo.GetTextElementEnumerator(run.Text);
                    while (elements.MoveNext())
                    {
                        var text = elements.GetTextElement();
                        var key = Font(format.CharacterShape, text, run.Strong, run.Emphasis);
                        var width = Width(key, text);
                        lineWidth += previousGap + width;
                        previousGap = width * key.Spacing / 100.0;
                        if (string.IsNullOrWhiteSpace(text))
                        {
                            minimum = Math.Max(minimum, wordWidth);
                            wordWidth = 0; wordGap = 0;
                        }
                        else
                        {
                            var keepWord = Language(text) == "Latin" ? format.KeepLatinWords : format.KeepOtherWords;
                            if (keepWord)
                            {
                                wordWidth += wordGap + width;
                                wordGap = previousGap;
                            }
                            else
                            {
                                minimum = Math.Max(minimum, wordWidth);
                                wordWidth = 0; wordGap = 0;
                                minimum = Math.Max(minimum, width);
                            }
                        }
                    }
                }
                minimum = Math.Max(minimum, wordWidth);
                preferred = Math.Max(preferred, lineWidth);
            }
            return (minimum, preferred);
        }

        private sealed record MeasuredText(double Width, double LastGap);

        private MeasuredText ReserveNative(PreviewTextRun run, CellFormat format)
        {
            if (run.Footnote is not null)
            {
                var noteShape = new XElement(format.CharacterShape);
                noteShape.SetAttributeValue("Height", Math.Max(1, (int)Math.Round((int)noteShape.Attribute("Height")! * 0.7)));
                return MeasurePlain("999)", noteShape, false, false);
            }
            var reference = run.CrossReference!;
            if (references is null)
                return MeasurePlain(reference.Kind switch { "figure_number" => "그림 999-999", "table_number" => "표 999-999", _ => "999.999.999." },
                    format.CharacterShape, run.Strong, run.Emphasis);
            var marker = "MD2HWP_WIDTH_" + Guid.NewGuid().ToString("N");
            var fragments = reference.Kind switch
            {
                "figure_number" => references.CreateFigureNumberFragments(document, 999, marker),
                "table_number" => references.CreateTableNumberFragments(document, 999, marker),
                _ => references.CreateHeadingNumberFragments(document, marker)
            };
            var width = 0.0;
            var previousGap = 0.0;
            foreach (var fragment in fragments)
            {
                var shape = document.Descendants("CHARSHAPE").Single(e =>
                    (string?)e.Attribute("Id") == (string?)fragment.Attribute("CharShape"));
                var text = string.Concat(fragment.Elements("CHAR").Select(e => e.Value))
                    .Replace(marker, reference.Kind != "heading_number" ? "999" : "999.999.999.", StringComparison.Ordinal);
                var measured = MeasurePlain(text, shape, run.Strong, run.Emphasis);
                if (text.Length != 0)
                {
                    width += previousGap + measured.Width;
                    previousGap = measured.LastGap;
                }
            }
            return new(width, previousGap);
        }

        internal double MeasureText(string text, XElement shape, bool strong, bool emphasis) =>
            MeasurePlain(text, shape, strong, emphasis).Width;

        private MeasuredText MeasurePlain(string text, XElement shape, bool strong, bool emphasis)
        {
            var width = 0.0;
            var previousGap = 0.0;
            var elements = StringInfo.GetTextElementEnumerator(text);
            while (elements.MoveNext())
            {
                var element = elements.GetTextElement();
                var key = Font(shape, element, strong, emphasis);
                var advance = Width(key, element);
                width += previousGap + advance;
                previousGap = advance * key.Spacing / 100.0;
            }
            return new(width, previousGap);
        }

        private FontKey Font(XElement shape, string text, bool strong, bool emphasis)
        {
            var language = Language(text);
            var fontId = (string?)shape.Element("FONTID")?.Attribute(language)
                ?? throw new InvalidDataException($"Table character format has no {language} font ID.");
            var face = document.Descendants("FONTFACE").Single(e => (string?)e.Attribute("Lang") == language);
            var definition = face.Elements("FONT").Single(e => (string?)e.Attribute("Id") == fontId);
            if ((string?)definition.Attribute("Type") is { } fontType &&
                !string.Equals(fontType, "ttf", StringComparison.OrdinalIgnoreCase))
                throw new InvalidDataException("Table automatic widths require a Windows TrueType/OpenType font; Hancom HFT fonts cannot be measured. Replace the table sample font.");
            var name = (string?)definition.Attribute("Name")
                ?? throw new InvalidDataException("Table font has no native face name.");
            var baseHeight = (int?)shape.Attribute("Height") ?? throw new InvalidDataException("Table font has no height.");
            var relativeSize = (int?)shape.Element("RELSIZE")?.Attribute(language) ?? 100;
            var ratio = (int?)shape.Element("RATIO")?.Attribute(language) ?? 100;
            var spacing = (int?)shape.Element("CHARSPACING")?.Attribute(language) ?? 0;
            if (baseHeight <= 0 || relativeSize <= 0 || ratio <= 0 || name.Length is 0 or > 31)
                throw new InvalidDataException("Table font size, horizontal scale or face name is invalid.");
            return new(name, checked((int)Math.Round(baseHeight * relativeSize / 100.0 * LogicalScale)),
                baseHeight, ratio, spacing, strong || shape.Element("BOLD") is not null,
                emphasis || shape.Element("ITALIC") is not null, (string?)shape.Attribute("UseFontSpace") == "true");
        }

        private double Width(FontKey key, string text)
        {
            if (widths.TryGetValue((key, text), out var cached)) return cached;
            if (!fonts.TryGetValue(key, out var font))
            {
                font = Native.CreateFontW(-key.Height, 0, 0, 0, key.Bold ? 700 : 400,
                    key.Italic ? 1u : 0, 0, 0, 1, 4, 0, 2, 0, key.Face);
                if (font == 0) throw new Win32Exception(Marshal.GetLastWin32Error(), $"Could not measure table font {key.Face}.");
                fonts.Add(key, font);
            }
            var previous = Native.SelectObject(device, font);
            if (previous == 0 || previous == -1)
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not select the table font.");
            var selectedFace = new StringBuilder(32);
            if (Native.GetTextFaceW(device, selectedFace.Capacity, selectedFace) == 0)
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not identify the table measurement font.");
            if (!string.Equals(selectedFace.ToString(), key.Face, StringComparison.OrdinalIgnoreCase))
                throw new InvalidDataException($"Table font {key.Face} is unavailable to Windows; GDI selected {selectedFace}. Install the font or change the table sample font.");
            // A tab in literal inline text uses a conservative four-space advance;
            // pipe table input ordinarily normalizes whitespace before this stage.
            var measuredText = text == "\t" ? "    " : text;
            if (!Native.GetTextExtentPoint32W(device, measuredText, measuredText.Length, out var size))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not measure table text.");
            var advance = text == " " && !key.UseFontSpace
                ? key.Height / (double)LogicalScale / 2 : size.Width / (double)LogicalScale;
            var width = Math.Max(0, advance * key.Ratio / 100.0);
            widths.Add((key, text), width);
            return width;
        }

        public void Dispose()
        {
            if (originalFont != 0) _ = Native.SelectObject(device, originalFont);
            foreach (var font in fonts.Values) _ = Native.DeleteObject(font);
            _ = Native.DeleteDC(device);
        }
    }

    private sealed record FontKey(string Face, int Height, int BaseHeight, int Ratio, int Spacing, bool Bold, bool Italic, bool UseFontSpace);

    private static string Language(string text)
    {
        var rune = Rune.GetRuneAt(text, 0).Value;
        if (rune is >= 0x1100 and <= 0x11ff or >= 0x3130 and <= 0x318f or >= 0xa960 and <= 0xa97f or >= 0xac00 and <= 0xd7ff)
            return "Hangul";
        if (rune is >= 0x3040 and <= 0x30ff or >= 0x31f0 and <= 0x31ff or >= 0xff66 and <= 0xff9f) return "Japanese";
        if (rune is >= 0x3400 and <= 0x9fff or >= 0xf900 and <= 0xfaff or >= 0x20000 and <= 0x3134f) return "Hanja";
        if (rune is <= 0x024f) return "Latin";
        if (rune is >= 0x2000 and <= 0x2bff) return "Symbol";
        return "Other";
    }

    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)] internal struct Size { internal int Width; internal int Height; }
        [DllImport("gdi32.dll", SetLastError = true)] internal static extern nint CreateCompatibleDC(nint device);
        [DllImport("gdi32.dll", SetLastError = true)] internal static extern nint GetCurrentObject(nint device, uint type);
        [DllImport("gdi32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
        internal static extern nint CreateFontW(int height, int width, int escapement, int orientation,
            int weight, uint italic, uint underline, uint strikeOut, uint characterSet,
            uint outputPrecision, uint clipPrecision, uint quality, uint pitchAndFamily, string face);
        [DllImport("gdi32.dll", SetLastError = true)] internal static extern nint SelectObject(nint device, nint selected);
        [DllImport("gdi32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool GetTextExtentPoint32W(nint device, string text, int count, out Size size);
        [DllImport("gdi32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
        internal static extern int GetTextFaceW(nint device, int count, StringBuilder face);
        [DllImport("gdi32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool DeleteObject(nint handle);
        [DllImport("gdi32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool DeleteDC(nint device);
    }
}
