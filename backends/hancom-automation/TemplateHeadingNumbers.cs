using System.Globalization;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal static class TemplateHeadingNumbers
{
    internal const string Tag = "{{md2hwp:num:heading1}}";
    private const string Marker = "MD2HWP_HEADING1_NUMBER_47C7C3F9";
    internal static int Start(IReadOnlyDictionary<string,string> metadata)
    {
        if (!metadata.TryGetValue("md2hwp-heading1-start",out var text)) return 1;
        if (text.Length == 0 || text.Any(c=>c is < '0' or > '9') ||
            !int.TryParse(text,NumberStyles.None,CultureInfo.InvariantCulture,out var number) || number < 1)
            throw new InvalidDataException("/metadata/md2hwp-heading1-start: expected a positive integer from 1 to 2147483647.");
        return number;
    }
    internal static PreviewOperation[] Track(IEnumerable<PreviewOperation> operations,int start)
    {
        int? chapter = null;
        return operations.Select(operation=> {
            if (operation.Kind == "text" && operation.ParagraphStyle == "heading1")
            {
                if (chapter == int.MaxValue) throw new InvalidDataException("Heading1 numbering exceeds 2147483647.");
                chapter = chapter is null ? start : chapter.Value + 1;
            }
            return operation with { Heading1Number = chapter };
        }).ToArray();
    }
    private static string NumberMarker(int level) => Marker.Replace("HEADING1", $"HEADING{level}", StringComparison.Ordinal);
    private static readonly Regex Markers = new(@"MD2HWP_HEADING([1-6])_NUMBER_47C7C3F9", RegexOptions.CultureInvariant);
    internal static string PublicTags(string text) => Markers.Replace(text, match => TaggedTemplateBinding.Tag("num:heading" + match.Groups[1].Value));
    internal static XDocument Prepare(XDocument source)
    {
        if (source.Descendants("P").Any(p => Markers.IsMatch(TaggedTemplateBinding.DirectText(p))))
            throw new InvalidDataException("Reserved internal heading number marker in template.");
        var document = new XDocument(source);
        for (var level = 1; level <= 6; level++)
        {
            var tag = TaggedTemplateBinding.Tag($"num:heading{level}");
            var replacement = NumberMarker(level);
            document = TemplateMetadata.Transform(document, tag, new Regex(Regex.Escape(tag)), _ => replacement);
        }
        return document;
    }
    internal static XElement[] Fill(IEnumerable<XElement> paragraphs,int? number)
    {
        var document=new XDocument(new XElement("ROOT",paragraphs.Select(p=>new XElement(p))));
        var result=TemplateMetadata.Transform(document,Marker,new Regex(Regex.Escape(Marker)),_=>
            number?.ToString(CultureInfo.InvariantCulture) ?? throw new InvalidDataException("num:heading1 requires a preceding heading1 in the manuscript."));
        return result.Root!.Elements().ToArray();
    }
    internal static bool ContainsNumberSlot(XElement paragraph) =>
        TaggedTemplateBinding.DirectText(paragraph).Contains(Tag, StringComparison.Ordinal) ||
        TaggedTemplateBinding.DirectText(paragraph).Contains(Marker, StringComparison.Ordinal);
    private static readonly Regex NumberTags = new(@"\{\{md2hwp:num:heading([1-6])\}\}|MD2HWP_HEADING([1-6])_NUMBER_47C7C3F9", RegexOptions.CultureInvariant);
    internal static string WithoutNumbers(string text, int level)
    {
        var clean = NumberTags.Replace(text, match =>
        {
            var target = int.Parse(match.Groups[1].Success ? match.Groups[1].Value : match.Groups[2].Value, CultureInfo.InvariantCulture);
            if (target > level) throw new InvalidDataException($"heading{level} scope cannot use num:heading{target}.");
            return "";
        });
        if (clean.Contains("{{md2hwp:num:", StringComparison.Ordinal)) throw new InvalidDataException("Unsupported or malformed heading number tag.");
        return clean;
    }
    internal static void FillParagraph(XElement paragraph, int level, IReadOnlyList<int> numbers)
    {
        WithoutNumbers(TaggedTemplateBinding.DirectText(paragraph), level);
        string Value(Match match) => numbers[int.Parse(match.Groups[1].Success ? match.Groups[1].Value : match.Groups[2].Value, CultureInfo.InvariantCulture) - 1].ToString(CultureInfo.InvariantCulture);
        TemplateMetadata.TransformParagraph(paragraph, "{{md2hwp:num:", NumberTags, Value);
        TemplateMetadata.TransformParagraph(paragraph, "MD2HWP_HEADING", NumberTags, Value);
    }
    internal static void RequireResolved(XDocument document)
    {
        if (document.Descendants("P").Any(p=>Markers.IsMatch(TaggedTemplateBinding.DirectText(p))))
            throw new InvalidDataException("Unresolved heading number tag: use own/ancestor numbers in heading scopes, or num:heading1 in figure/table captions and object-reference samples.");
    }
}
