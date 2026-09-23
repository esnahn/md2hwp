using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal static class HwpMarkup
{
    public static XDocument Parse(string xml)
    {
        var document = XDocument.Parse(xml, LoadOptions.PreserveWhitespace);
        // CHAR payloads can consist only of spaces between formatted runs.
        // Ignore XML indentation elsewhere, never manuscript whitespace.
        foreach (var text in document.DescendantNodes().OfType<XText>()
                     .Where(text => text.Parent?.Name.LocalName != "CHAR" &&
                                    string.IsNullOrWhiteSpace(text.Value)).ToArray())
            text.Remove();
        return document;
    }
}
