using System.Globalization;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// IR math becomes native Hancom EQUATION objects. HWPML import does not lay out
// equations, so geometry comes from natively created equations; the template
// owns only the display paragraph and its font size.
internal sealed class TemplateDisplayMath
{
    internal const string InlineMarkerPrefix = "MD2HWP_INLINE_MATH_";
    internal const string DisplayMarkerPrefix = "MD2HWP_DISPLAY_MATH_";
    internal const string SizeSetting = "display_math.fontsize-pt";
    internal static readonly string Slot = TaggedTemplateBinding.Tag("slot:display_math");
    private static readonly Regex SizeValue = new(@"^[0-9]{1,9}(\.[0-9]{1,2})?$", RegexOptions.CultureInvariant);

    private readonly XDocument source;
    private readonly XElement sample;

    private TemplateDisplayMath(XDocument source, XElement sample, int baseUnit)
    {
        this.source = source;
        this.sample = sample;
        DisplayBaseUnit = baseUnit;
    }

    internal int DisplayBaseUnit { get; }

    internal static (TemplateDisplayMath Layout, XDocument Document) Lower(XDocument source)
    {
        var document = new XDocument(source);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document).ToArray();
        int Find(string token)
        {
            var matches = roots.Select((p, i) => (p, i))
                .Where(x => TaggedTemplateBinding.DirectText(x.p) == TaggedTemplateBinding.Tag(token)).ToArray();
            if (matches.Length != 1)
                throw new InvalidDataException($"Expected one {token} declaration; found {matches.Length}. " +
                    "Regenerate the template with init-template or explicitly adopt the display_math declarations.");
            return matches[0].i;
        }
        var templateBegin = Find("begin:template");
        var templateEnd = Find("end:template");
        var prefix = TaggedTemplateBinding.Prefix + SizeSetting + ":";
        var settings = roots.Select((p, i) => (p, i, text: TaggedTemplateBinding.DirectText(p)))
            .Where(x => x.text.StartsWith(prefix, StringComparison.Ordinal)).ToArray();
        if (settings.Length != 1)
            throw new InvalidDataException($"Expected one {SizeSetting} setting; found {settings.Length}. " +
                "Regenerate the template with init-template or explicitly adopt the display_math declarations.");
        var setting = settings[0];
        if (!setting.text.EndsWith("}}", StringComparison.Ordinal) || !PlainText(setting.p))
            throw new InvalidDataException($"{SizeSetting} must be a plain root paragraph.");
        var value = setting.text[prefix.Length..^2];
        if (!SizeValue.IsMatch(value) ||
            !decimal.TryParse(value, NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out var points) ||
            points <= 0 || points * 100 > int.MaxValue)
            throw new InvalidDataException($"{SizeSetting} must be a positive decimal point size with at most two decimal places.");
        var begin = Find("begin:display_math");
        var end = Find("end:display_math");
        if (!(templateBegin < setting.i && setting.i < begin && begin < end && end < templateEnd))
            throw new InvalidDataException($"{SizeSetting} must precede begin:display_math inside the template definitions.");
        if (end != begin + 2)
            throw new InvalidDataException("display_math range must contain exactly one sample paragraph.");
        var paragraph = roots[begin + 1];
        RequireSample(paragraph, source);
        var layout = new TemplateDisplayMath(new XDocument(source), new XElement(paragraph),
            checked((int)(points * 100)));
        foreach (var index in new[] { setting.i, begin, begin + 1, end }) roots[index].Remove();
        return (layout, document);
    }

    private static bool PlainText(XElement paragraph) =>
        paragraph.Elements().All(t => t.Name.LocalName == "TEXT" &&
            t.Elements().All(c => c.Name.LocalName == "CHAR" && !c.HasElements));

    // The sample owns paragraph formatting, literal affixes, native tabs and one
    // optional Equation AUTONUM. The number shows the preceding equation's
    // counter, so it must follow the slot.
    private static void RequireSample(XElement paragraph, XDocument source)
    {
        const string message = "display_math sample must be one plain paragraph containing slot:display_math, " +
            "with only text, native tabs and at most one Equation AUTONUM after the slot.";
        var text = TaggedTemplateBinding.DirectText(paragraph);
        var parts = text.Split(Slot, StringSplitOptions.None);
        if (parts.Length != 2 || text.Replace(Slot, "", StringComparison.Ordinal).Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
            throw new InvalidDataException(message);
        var children = paragraph.Elements().ToArray();
        if (children.Length == 0 || children.Any(e => e.Name.LocalName != "TEXT"))
            throw new InvalidDataException(message);
        var controls = children.SelectMany(t => t.Elements()).ToArray();
        if (controls.Any(e => e.Name.LocalName is not ("CHAR" or "AUTONUM")) ||
            controls.Where(e => e.Name.LocalName == "CHAR").SelectMany(c => c.Elements()).Any(e => e.Name.LocalName != "TAB"))
            throw new InvalidDataException(message);
        var numbers = controls.Where(e => e.Name.LocalName == "AUTONUM").ToArray();
        if (numbers.Length > 1 || numbers.Any(n => (string?)n.Attribute("NumberType") != "Equation"))
            throw new InvalidDataException(message);
        var slotCharacter = SlotCharacter(paragraph) ?? throw new InvalidDataException(
            "slot:display_math must lie in one text run without crossing tabs or controls.");
        if (numbers.Length == 1 && numbers[0].IsBefore(slotCharacter))
            throw new InvalidDataException("The Equation AUTONUM must follow slot:display_math; Hancom shows the preceding equation's number.");
        var shape = (string?)paragraph.Attribute("ParaShape");
        var definition = source.Descendants("PARASHAPE").SingleOrDefault(p => (string?)p.Attribute("Id") == shape);
        if (definition is not null && (string?)definition.Attribute("HeadingType") is { } heading && heading != "None")
            throw new InvalidDataException("display_math sample must not use list or outline numbering.");
    }

    private static XText? SlotCharacter(XElement paragraph) => paragraph.Elements("TEXT").Elements("CHAR")
        .SelectMany(c => c.Nodes().OfType<XText>()).SingleOrDefault(t => t.Value.Contains(Slot, StringComparison.Ordinal));

}
