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
    internal static XDocument Prepare(XDocument source)
    {
        if (source.Descendants("P").Any(p=>TaggedTemplateBinding.DirectText(p).Contains(Marker,StringComparison.Ordinal)))
            throw new InvalidDataException("Reserved internal heading1 number marker in template.");
        return TemplateMetadata.Transform(source,Tag,new Regex(Regex.Escape(Tag)),_=>Marker);
    }
    internal static XElement[] Fill(IEnumerable<XElement> paragraphs,int? number)
    {
        var document=new XDocument(new XElement("ROOT",paragraphs.Select(p=>new XElement(p))));
        var result=TemplateMetadata.Transform(document,Marker,new Regex(Regex.Escape(Marker)),_=>
            number?.ToString(CultureInfo.InvariantCulture) ?? throw new InvalidDataException("num:heading1 requires a preceding heading1 in the manuscript."));
        return result.Root!.Elements().ToArray();
    }
    internal static void RequireResolved(XDocument document)
    {
        if (document.Descendants("P").Any(p=>TaggedTemplateBinding.DirectText(p).Contains(Marker,StringComparison.Ordinal)))
            throw new InvalidDataException("num:heading1 is allowed only inside heading blocks and figure captions.");
    }
}
