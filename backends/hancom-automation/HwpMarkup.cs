using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal static class HwpMarkup
{
    internal static XDocument ReadDocument(object automation
#if DEBUG
        , [System.Runtime.CompilerServices.CallerMemberName] string caller = "",
        [System.Runtime.CompilerServices.CallerFilePath] string file = ""
#endif
        )
    {
#if DEBUG
        return RenderProfile.ReadDocument(automation, caller, file);
#else
        dynamic hwp = automation;
        return Parse((string)hwp.GetTextFile("HWPML2X", ""));
#endif
    }

    internal static XDocument ReadBlock(object automation
#if DEBUG
        , [System.Runtime.CompilerServices.CallerMemberName] string caller = "",
        [System.Runtime.CompilerServices.CallerFilePath] string file = ""
#endif
        )
    {
#if DEBUG
        return RenderProfile.ReadBlock(automation, caller, file);
#else
        dynamic hwp = automation;
        return Parse((string)hwp.GetTextFile("HWPML2X", "saveblock"));
#endif
    }

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
