using System.Globalization;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// One native equation per distinct SCRIPT and editor font size (1/100 pt).
internal sealed record EquationKey(string Script, int BaseUnit);

// IR math becomes native Hancom EQUATION objects. HWPML import does not lay out
// equations, so geometry comes from natively created equations; the template
// owns only the display paragraph and its font size.
internal sealed class TemplateDisplayMath
{
    internal const string InlineMarkerPrefix = "MD2HWP_INLINE_MATH_";
    internal const string DisplayMarkerPrefix = "MD2HWP_DISPLAY_MATH_";
    internal const string SizeSetting = "display_math.fontsize-pt";
    internal static readonly string Slot = TaggedTemplateBinding.Tag("slot:display_math");
    private static readonly Regex InlineMarker = new(InlineMarkerPrefix + "[0-9a-f]{32}",
        RegexOptions.CultureInvariant | RegexOptions.NonBacktracking);
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

    internal XDocument Attach(XDocument rendered, IrPreviewPlan plan,
        Func<IReadOnlyList<EquationKey>, IReadOnlyDictionary<EquationKey, XElement>> measure)
    {
        var result = new XDocument(rendered);
        var inlineScripts = FigureReferenceContract.ReadRuns(plan.Operations).Where(r => r.Math is not null)
            .ToDictionary(r => r.Text, r => r.Math!, StringComparer.Ordinal);
        var displayScripts = plan.Operations.Where(o => o.DisplayMath is not null)
            .ToDictionary(o => o.Lines.Single(), o => o.DisplayMath!, StringComparer.Ordinal);
        if (inlineScripts.Count == 0 && displayScripts.Count == 0) return result;

        var displays = new List<(XElement Placeholder, EquationKey Key)>();
        foreach (var (marker, script) in displayScripts)
        {
            var matches = result.Descendants("P").Where(p => TaggedTemplateBinding.DirectText(p) == marker).ToArray();
            if (matches.Length != 1 || matches[0].Parent?.Name.LocalName != "SECTION")
                throw new InvalidOperationException("A generated display math paragraph was lost before native insertion.");
            displays.Add((matches[0], new EquationKey(script, DisplayBaseUnit)));
        }
        var inlines = new List<(XElement Paragraph, string Marker, EquationKey Key)>();
        foreach (var paragraph in result.Descendants("P").ToArray())
        {
            var found = InlineMarker.Matches(TaggedTemplateBinding.DirectText(paragraph))
                .Where(m => inlineScripts.ContainsKey(m.Value)).ToArray();
            if (found.Length == 0) continue;
            if (paragraph.Ancestors().Any(e => e.Name.LocalName is "HEADER" or "FOOTER" or "MASTERPAGE" or "CAPTION" or "CELL"))
                throw new InvalidDataException("Inline math cannot be placed in headers, footers, master pages, captions or table cells.");
            var unit = StyleBaseUnit(paragraph, result);
            inlines.AddRange(found.Select(m => (paragraph, m.Value, new EquationKey(inlineScripts[m.Value], unit))));
        }
        if (inlines.Select(i => i.Marker).Distinct(StringComparer.Ordinal).Count() != inlineScripts.Count)
            throw new InvalidOperationException("A generated inline math marker was lost before native insertion.");

        var keys = displays.Select(d => d.Key).Concat(inlines.Select(i => i.Key)).Distinct().ToArray();
        var natives = measure(keys);
        foreach (var key in keys)
            if (!natives.ContainsKey(key)) throw new InvalidOperationException("Missing native equation geometry.");

        foreach (var (placeholder, key) in displays)
        {
            var paragraph = TemplateHeadingBlocks.ImportParagraph(sample, source, result);
            var slot = SlotCharacter(paragraph)!;
            var character = slot.Parent!;
            var parts = slot.Value.Split(Slot);
            var after = CharacterAfter(character, slot, parts[1]);
            // Hancom exports no empty text node, e.g. after a leading tab.
            if (parts[0].Length == 0) slot.Remove(); else slot.Value = parts[0];
            character.AddAfterSelf(Equation(natives[key], "Equation", result), after);
            Clean(paragraph);
            placeholder.ReplaceWith(paragraph);
        }
        foreach (var group in inlines.GroupBy(i => i.Paragraph))
            ReplaceInline(group.Key, group.ToDictionary(i => i.Marker, i => Equation(natives[i.Key], "None", result), StringComparer.Ordinal));
        AllocateOrders(result);
        RecalculateNumbers(result);
        return result;
    }

    // Split one CHAR at the slot: text and tabs after it move to a new CHAR.
    private static XElement CharacterAfter(XElement character, XText slot, string suffix)
    {
        var after = new XElement("CHAR");
        if (suffix.Length > 0) after.Add(new XText(suffix));
        foreach (var node in slot.NodesAfterSelf().ToArray()) { node.Remove(); after.Add(node); }
        return after;
    }

    private static XElement Equation(XElement native, string numbering, XDocument destination)
    {
        var equation = new XElement(native);
        var shape = equation.Element("SHAPEOBJECT") ?? throw new InvalidOperationException("Native equation has no shape object.");
        shape.SetAttributeValue("NumberingType", numbering);
        shape.AddAnnotation(new GeneratedEquation());
        TemplateHeadingBlocks.ReassignCopiedIdentities(destination, [equation]);
        return equation;
    }

    // Inline equations use the editor size of their paragraph style's character format.
    private static int StyleBaseUnit(XElement paragraph, XDocument document)
    {
        var style = (string?)paragraph.Attribute("Style");
        var definition = document.Descendants("STYLE").SingleOrDefault(s =>
            (string?)s.Attribute("Id") == style && (string?)s.Attribute("Type") == "Para")
            ?? throw new InvalidDataException("Inline math paragraph has no native paragraph style.");
        var characterId = (string?)definition.Attribute("CharShape");
        var shape = document.Descendants("CHARSHAPE").SingleOrDefault(s => (string?)s.Attribute("Id") == characterId)
            ?? throw new InvalidDataException("Inline math paragraph style has no character format.");
        if (!int.TryParse((string?)shape.Attribute("Height"), NumberStyles.None, CultureInfo.InvariantCulture, out var height) || height <= 0)
            throw new InvalidDataException("Inline math paragraph style has no positive character size.");
        return height;
    }

    private static void ReplaceInline(XElement paragraph, IReadOnlyDictionary<string, XElement> equations)
    {
        // Split mixed payloads so tabs and line breaks stay separate atoms.
        foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").Where(c => c.HasElements).ToArray())
            character.ReplaceWith(character.Nodes().Select(n => new XElement("CHAR",
                n is XText text ? new XText(text.Value) : new XElement((XElement)n))).ToArray());
        var atoms = paragraph.Elements("TEXT").SelectMany(t => t.Elements())
            .SelectMany(e => e.Name.LocalName == "CHAR" && !e.HasElements
                ? e.Value.Select((c, i) => (Element: e, Offset: i, Character: c))
                : [(Element: e, Offset: 0, Character: '\0')]).ToArray();
        var value = new string(atoms.Select(a => a.Character).ToArray());
        foreach (var match in InlineMarker.Matches(value).Where(m => equations.ContainsKey(m.Value)).OrderByDescending(m => m.Index))
        {
            var first = atoms[match.Index];
            var last = atoms[match.Index + match.Length - 1];
            if (first.Element != last.Element)
                throw new InvalidOperationException("An inline math marker crossed a character run.");
            var text = first.Element.Value;
            first.Element.Value = text[..first.Offset];
            first.Element.AddAfterSelf(equations[match.Value], new XElement("CHAR", text[(last.Offset + 1)..]));
        }
        Clean(paragraph);
    }

    private static void Clean(XElement paragraph)
    {
        paragraph.Elements("TEXT").Elements("CHAR").Where(c => !c.HasElements && c.Value.Length == 0).Remove();
        paragraph.Elements("TEXT").Where(t => !t.HasElements).Remove();
        foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").ToArray())
            if (character.PreviousNode is XElement previous && previous.Name.LocalName == "CHAR" &&
                !previous.HasAttributes && !character.HasAttributes)
            { previous.Add(character.Nodes().ToArray()); character.Remove(); }
    }

    // Generated equations take fresh stacking orders above every existing object.
    private static void AllocateOrders(XDocument document)
    {
        var shapes = document.Descendants("SHAPEOBJECT").ToArray();
        var next = shapes.Select(s => (int?)s.Attribute("ZOrder") ?? 0).DefaultIfEmpty(0).Max();
        foreach (var equation in document.Descendants("EQUATION"))
        {
            var shape = equation.Element("SHAPEOBJECT")!;
            if (shape.Annotation<GeneratedEquation>() is null) continue;
            shape.SetAttributeValue("ZOrder", next = checked(next + 1));
        }
    }

    // Hancom's Equation AUTONUM shows the counter of the latest preceding
    // equation-numbered object. NEWNUM sets the next object's number.
    internal static void RecalculateNumbers(XDocument document)
    {
        var current = ((int?)document.Descendants("DOCSETTING").Elements("BEGINNUMBER").SingleOrDefault()?.Attribute("Equation") ?? 1) - 1;
        foreach (var element in document.Descendants("SECTION").SelectMany(s => s.Descendants()))
        {
            if (element.Ancestors().Any(a => a.Name.LocalName is "HEADER" or "FOOTER" or "MASTERPAGE")) continue;
            if (element.Name.LocalName == "NEWNUM" && (string?)element.Attribute("NumberType") == "Equation")
                current = (int)element.Attribute("Number")! - 1;
            else if (element.Name.LocalName == "EQUATION" &&
                (string?)element.Element("SHAPEOBJECT")?.Attribute("NumberingType") == "Equation")
                current = checked(current + 1);
            else if (element.Name.LocalName == "AUTONUM" && (string?)element.Attribute("NumberType") == "Equation")
                element.SetAttributeValue("Number", current);
        }
    }

    internal sealed class GeneratedEquation;
}
