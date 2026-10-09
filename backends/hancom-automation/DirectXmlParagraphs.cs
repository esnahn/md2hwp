using System.Xml;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Compose the same STYLE defaults that StyleEx applies in the ordinary COM
// renderer. A tagged sample's direct paragraph/character overrides are not
// substituted for its style. Numbering and continuation changes belong to the
// native-list composer, and note/reference placeholders remain literal here.
internal sealed class DirectXmlParagraphs(XDocument destination, AuriPreviewStyleBindings styles)
{
    private readonly Dictionary<(string Baseline, bool Strong, bool Emphasis), string> characterIds = [];

    internal IReadOnlyList<XElement> Create(PreviewOperation operation)
    {
        if (operation.Kind is not ("text" or "table"))
            throw new InvalidOperationException($"Direct paragraph composition does not support {operation.Kind}.");
        if (operation.ParagraphStyle is not { } role || operation.Lines.Count == 0 ||
            operation.FormattedLines is not { } lines || lines.Count != operation.Lines.Count)
            throw new InvalidOperationException($"Missing direct paragraph content/style for {operation.Label}.");
        for (var line = 0; line < lines.Count; line++)
            if (!string.Equals(string.Concat(lines[line].Select(run => run.Text)), operation.Lines[line], StringComparison.Ordinal))
                throw new InvalidOperationException($"Formatted runs do not match plain line {line} for {operation.Label}.");
        if (styles.Profile.PreserveParagraphLineBreaks) return [Create(role, lines)];
        return lines.Select(line => Create(role, [line])).ToArray();
    }

    internal XElement Create(string role, IReadOnlyList<IReadOnlyList<PreviewTextRun>> lines)
    {
        var native = styles.Resolve(role);
        var definition = destination.Descendants("STYLE").SingleOrDefault(style =>
            (string?)style.Attribute("Id") == native.Id.ToString(System.Globalization.CultureInfo.InvariantCulture) &&
            (string?)style.Attribute("Type") == "Para")
            ?? throw new InvalidOperationException($"Missing direct paragraph STYLE for {role}.");
        var paragraphId = (string?)definition.Attribute("ParaShape")
            ?? throw new InvalidOperationException($"Direct paragraph STYLE {role} has no paragraph format.");
        if (!destination.Descendants("PARASHAPE").Any(shape => (string?)shape.Attribute("Id") == paragraphId))
            throw new InvalidOperationException($"Unknown direct paragraph format {paragraphId}.");
        var baseline = (string?)definition.Attribute("CharShape")
            ?? throw new InvalidOperationException($"Direct paragraph STYLE {role} has no character format.");
        var paragraph = new XElement("P", new XAttribute("ParaShape", paragraphId), new XAttribute("Style", native.Id));
        for (var line = 0; line < lines.Count; line++)
        {
            if (line > 0) Add(paragraph, baseline, new XElement("LINEBREAK"));
            foreach (var run in lines[line])
            {
                if (run.Text.IndexOfAny(['\r', '\n']) >= 0)
                    throw new InvalidOperationException("A direct paragraph run cannot contain CR/LF; use separate lines.");
                XmlConvert.VerifyXmlChars(run.Text);
                var character = CharacterId(baseline, run.Strong, run.Emphasis);
                var pieces = run.Text.Split('\t');
                for (var piece = 0; piece < pieces.Length; piece++)
                {
                    if (piece > 0) Add(paragraph, character, new XElement("TAB"));
                    if (pieces[piece].Length > 0) Add(paragraph, character, new XText(pieces[piece]));
                }
            }
        }
        // Keep the editing format even for an empty generated paragraph.
        if (!paragraph.HasElements) Add(paragraph, baseline, new XText(""));
        return paragraph;
    }

    private string CharacterId(string baselineId, bool strong, bool emphasis)
    {
        var key = (baselineId, strong, emphasis);
        if (characterIds.TryGetValue(key, out var existing)) return existing;
        var baseline = destination.Descendants("CHARSHAPE").SingleOrDefault(shape =>
            (string?)shape.Attribute("Id") == baselineId)
            ?? throw new InvalidOperationException($"Unknown direct character format {baselineId}.");
        var definition = new XElement(baseline);
        definition.Attribute("Id")!.Remove();
        ApplyMarks(definition, strong, emphasis);
        var table = baseline.Parent!;
        var match = table.Elements("CHARSHAPE").FirstOrDefault(candidate =>
        {
            var copy = new XElement(candidate); copy.Attribute("Id")?.Remove();
            return XNode.DeepEquals(copy, definition);
        });
        if (match is null)
        {
            definition.SetAttributeValue("Id", checked(table.Elements("CHARSHAPE").Max(shape => (int)shape.Attribute("Id")!) + 1));
            table.Add(definition);
            table.SetAttributeValue("Count", table.Elements("CHARSHAPE").Count());
            match = definition;
        }
        var id = (string)match.Attribute("Id")!;
        characterIds.Add(key, id);
        return id;
    }

    // Hancom exports the language metrics first, then ITALIC and BOLD, followed
    // by the remaining character effects. Apply this only to owned new variants;
    // existing template definitions and comparison inputs remain untouched.
    internal static void ApplyMarks(XElement definition, bool strong, bool emphasis)
    {
        if (strong && definition.Element("BOLD") is null) definition.Add(new XElement("BOLD"));
        if (emphasis && definition.Element("ITALIC") is null) definition.Add(new XElement("ITALIC"));
        OrderMarks(definition);
    }

    internal static void OrderMarks(XElement definition)
    {
        var italic = definition.Element("ITALIC");
        var bold = definition.Element("BOLD");
        if (italic is null && bold is null) return;
        italic?.Remove();
        bold?.Remove();
        var effects = new[] { italic, bold }.OfType<XElement>().ToArray();
        var following = definition.Elements().FirstOrDefault(element => element.Name.LocalName is not
            ("FONTID" or "RATIO" or "CHARSPACING" or "RELSIZE" or "CHAROFFSET"));
        if (following is not null) following.AddBeforeSelf(effects);
        else definition.Add(effects);
    }

    private static void Add(XElement paragraph, string characterId, XNode payload)
    {
        var run = paragraph.Elements("TEXT").LastOrDefault();
        if (run is null || (string?)run.Attribute("CharShape") != characterId)
        {
            run = new XElement("TEXT", new XAttribute("CharShape", characterId));
            paragraph.Add(run);
        }
        var character = run.Elements("CHAR").LastOrDefault();
        if (character is null)
        {
            character = new XElement("CHAR");
            run.Add(character);
        }
        character.Add(payload);
    }
}
