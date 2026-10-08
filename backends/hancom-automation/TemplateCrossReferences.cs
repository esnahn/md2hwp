using System.Globalization;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Paragraph prototypes own the wording and slots of native number references.
// Native fields are inserted later by the adapter at the number slots.
internal sealed class TemplateCrossReferences(XDocument source, XElement figureNumber, XElement? tableNumber = null, IReadOnlyDictionary<int, XElement>? headings = null)
{
    internal const string FigureNumberRole = "ref.figure.number";
    internal const string TableNumberRole = "ref.table.number";
    internal static readonly string TableNumberSlot = TaggedTemplateBinding.Tag("slot:" + TableNumberRole);
    internal static string HeadingRole(int level) => $"ref.heading{level}.number";
    internal static readonly string FigureNumberSlot = TaggedTemplateBinding.Tag("slot:" + FigureNumberRole);

    private static readonly Regex Tags = new(@"\{\{md2hwp:([^{}]+)\}\}", RegexOptions.CultureInvariant);
    private static readonly Regex ChapterSlot = new(Regex.Escape(TemplateHeadingNumbers.Tag), RegexOptions.CultureInvariant);

    internal static (TemplateCrossReferences Layout, XDocument Document) Lower(XDocument source)
    {
        var document = new XDocument(source);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document).ToArray();
        var roles = new[] { FigureNumberRole, TableNumberRole }.Concat(Enumerable.Range(1, 6).Select(HeadingRole)).ToArray();
        var permitted = roles.SelectMany(role => new[] { "begin:" + role, "end:" + role, "slot:" + role })
            .ToHashSet(StringComparer.Ordinal);
        foreach (var paragraph in document.Descendants("P"))
        foreach (Match token in Tags.Matches(TaggedTemplateBinding.DirectText(paragraph)))
        {
            var name = token.Groups[1].Value;
            if (name.Split(':').Any(part => part.StartsWith("ref.", StringComparison.Ordinal)) && !permitted.Contains(name))
                throw new InvalidDataException($"Unsupported template cross-reference declaration '{name}'. Use ref.figure.number, ref.table.number and ref.heading1.number through ref.heading6.number. Replace old ref.heading.number with six numbered heading reference blocks; other reference types and page references are reserved.");
        }

        int Find(string token)
        {
            var matches = roots.Select((paragraph, index) => (paragraph, index))
                .Where(item => TaggedTemplateBinding.DirectText(item.paragraph) == TaggedTemplateBinding.Tag(token)).ToArray();
            if (matches.Length != 1)
                throw new InvalidDataException($"Expected one root declaration {token}; found {matches.Length}. Regenerate the template with init-template or explicitly add the required reference prototypes to the edited template.");
            return matches[0].index;
        }

        var templateBegin = Find("begin:template");
        var templateEnd = Find("end:template");
        var accepted = new HashSet<XElement>();
        var prototypes = new Dictionary<string, XElement>(StringComparer.Ordinal);
        foreach (var role in roles)
        {
            var first = Find("begin:" + role);
            var last = Find("end:" + role);
            if (first <= templateBegin || last >= templateEnd || last != first + 2)
                throw new InvalidDataException($"{role} requires exactly one root sample paragraph inside begin:template/end:template.");
            var sample = roots[first + 1];
            foreach (var paragraph in new[] { roots[first], sample, roots[last] })
            {
                RequirePlain(paragraph, role);
                accepted.Add(paragraph);
            }
            var sampleText = TemplateHeadingNumbers.PublicTags(TaggedTemplateBinding.DirectText(sample));
            var headingLevel = Array.FindIndex(Enumerable.Range(1, 6).Select(HeadingRole).ToArray(), item => item == role) + 1;
            if (headingLevel > 0)
            {
                if (headingLevel == 1 ? !TemplateHeadingNumbers.ContainsNumberSlot(sample) :
                    !sampleText.Contains(TaggedTemplateBinding.Tag($"num:heading{headingLevel}"), StringComparison.Ordinal))
                    throw new InvalidDataException($"{role} sample requires num:heading{headingLevel}.");
                foreach (Match token in Tags.Matches(sampleText))
                    if (!Enumerable.Range(1, headingLevel).Any(level => token.Value == TaggedTemplateBinding.Tag($"num:heading{level}")))
                        throw new InvalidDataException($"Unsupported tag '{token.Value}' in {role} sample.");
                var remainder = Tags.Replace(sampleText, "");
                if (remainder.Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
                    throw new InvalidDataException($"Malformed template tag in {role} sample.");
            }
            else
            {
                var slot = TaggedTemplateBinding.Tag("slot:" + role);
                var numberSlot = new Regex(Regex.Escape(slot), RegexOptions.CultureInvariant);
                if (numberSlot.Matches(sampleText).Count != 1)
                    throw new InvalidDataException($"{role} sample requires exactly one slot:{role}.");
                foreach (Match token in Tags.Matches(sampleText))
                    if (token.Value != slot && token.Value != TemplateHeadingNumbers.Tag)
                        throw new InvalidDataException($"Unsupported tag '{token.Value}' in {role} sample.");
                if (numberSlot.Replace(ChapterSlot.Replace(sampleText, ""), "").Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
                    throw new InvalidDataException($"Malformed template tag in {role} sample.");
            }
            prototypes.Add(role, new XElement(sample));
        }
        foreach (var paragraph in document.Descendants("P").Where(paragraph => !accepted.Contains(paragraph)))
        foreach (Match token in Tags.Matches(TaggedTemplateBinding.DirectText(paragraph)))
            if (permitted.Contains(token.Groups[1].Value))
                throw new InvalidDataException("Reference declarations and number slots are allowed only in their single root prototypes inside template definitions.");
        foreach (var paragraph in accepted) paragraph.Remove();
        return (new TemplateCrossReferences(new XDocument(source), prototypes[FigureNumberRole], prototypes[TableNumberRole], Enumerable.Range(1, 6).ToDictionary(level => level, level => prototypes[HeadingRole(level)])), document);
    }

    internal XElement[] CreateFigureNumberFragments(XDocument destination, int? targetHeading1Number, string nativeControlMarker, XElement? context = null) =>
        CreateFragments(destination, figureNumber, FigureNumberSlot, nativeControlMarker, targetHeading1Number, true, context);

    internal XElement[] CreateTableNumberFragments(XDocument destination, int? targetHeading1Number, string nativeControlMarker, XElement? context = null) =>
        CreateFragments(destination, tableNumber ?? throw new InvalidDataException("Missing required ref.table.number prototype. Regenerate the template with init-template."),
            TableNumberSlot, nativeControlMarker, targetHeading1Number, true, context);

    internal string HeadingText(int level, IReadOnlyList<int> numbers)
    {
        if (level is < 1 or > 6 || numbers.Count != 6 || headings is null || !headings.TryGetValue(level, out var sample))
            throw new InvalidDataException("Missing heading reference block; add ref.heading1.number through ref.heading6.number or regenerate the template.");
        var text = TemplateHeadingNumbers.PublicTags(TaggedTemplateBinding.DirectText(TemplateHeadingNumbers.Fill([sample], numbers[0])[0]));
        for (var index = 0; index < level; index++)
            text = text.Replace(TaggedTemplateBinding.Tag($"num:heading{index + 1}"), numbers[index].ToString(CultureInfo.InvariantCulture), StringComparison.Ordinal);
        if (text.Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal)) throw new InvalidDataException("Unresolved heading reference number tag.");
        return text;
    }

    private XElement[] CreateFragments(XDocument destination, XElement sample, string slot, string nativeControlMarker,
        int? targetHeading1Number, bool allowChapterNumber, XElement? context)
    {
        if (string.IsNullOrWhiteSpace(nativeControlMarker) || nativeControlMarker.Any(char.IsControl) ||
            nativeControlMarker.Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
            throw new InvalidDataException("A native number reference requires a nonempty, single-line native control marker.");
        if (source.Descendants("P").Concat(destination.Descendants("P"))
            .Any(paragraph => TaggedTemplateBinding.DirectText(paragraph).Contains(nativeControlMarker, StringComparison.Ordinal)))
            throw new InvalidDataException("The native reference control marker must be unique.");
        // The sample owns wording and slots; the source inline owns formatting.
        // Do not import sample style definitions when rendering a contextual reference.
        var imported = context is null ? TemplateHeadingBlocks.ImportParagraph(sample, source, destination) : new XElement(sample);
        if (context is not null)
            foreach (var text in imported.Elements("TEXT")) text.ReplaceAttributes(context.Attributes());
        XElement[] filled = allowChapterNumber ? TemplateHeadingNumbers.Fill([imported], targetHeading1Number) : [imported];
        var instance = new XDocument(new XElement("ROOT", filled));
        if (allowChapterNumber)
            instance = TemplateMetadata.Transform(instance, TemplateHeadingNumbers.Tag, ChapterSlot, _ =>
                targetHeading1Number?.ToString(CultureInfo.InvariantCulture)
                    ?? throw new InvalidDataException("num:heading1 in an object reference requires the target object to follow a heading1."));
        instance = TemplateMetadata.Transform(instance, slot, new Regex(Regex.Escape(slot), RegexOptions.CultureInvariant), _ => nativeControlMarker);
        return instance.Root!.Element("P")!.Elements("TEXT").Select(text => new XElement(text)).ToArray();
    }

    private static void RequirePlain(XElement paragraph, string role)
    {
        if (!paragraph.Elements("TEXT").Any() || paragraph.Elements().Any(element => element.Name.LocalName != "TEXT") ||
            paragraph.Nodes().OfType<XText>().Any(text => !string.IsNullOrWhiteSpace(text.Value)) ||
            paragraph.Elements("TEXT").Any(text => text.Nodes().OfType<XText>().Any(value => !string.IsNullOrWhiteSpace(value.Value)) || !text.Elements("CHAR").Any() ||
                text.Elements().Any(element => element.Name.LocalName != "CHAR" || element.HasElements)) ||
            TaggedTemplateBinding.DirectText(paragraph).Any(char.IsControl) ||
            paragraph.Attributes().Any(attribute => (attribute.Name.LocalName is "PageBreak" or "ColumnBreak") && attribute.Value != "false"))
            throw new InvalidDataException($"{role} boundaries and sample must be plain, single-line root paragraphs without native controls or page/column breaks.");
    }
}
