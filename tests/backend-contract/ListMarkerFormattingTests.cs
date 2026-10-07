using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class ListMarkerFormattingTests
{
    internal static void Run()
    {
        var source = XDocument.Parse("""
            <HWPML><HEAD>
            <CHARSHAPELIST><CHARSHAPE Id="4"/><CHARSHAPE Id="8"><ITALIC/></CHARSHAPE></CHARSHAPELIST>
            <PARASHAPELIST Count="2"><PARASHAPE Id="0" Heading="1"/><PARASHAPE Id="1" Heading="1"/></PARASHAPELIST>
            <BULLETLIST Count="1"><BULLET Id="1" Char="●"><PARAHEAD/></BULLET></BULLETLIST>
            <NUMBERINGLIST Count="1"><NUMBERING Id="1" Start="1"><PARAHEAD Level="1">^1.</PARAHEAD><PARAHEAD Level="2" CharShape="8">^2)</PARAHEAD></NUMBERING></NUMBERINGLIST>
            </HEAD><BODY><SECTION>
            <P ParaShape="0"><TEXT CharShape="4"><CHAR>{{md2hwp:list.bullet}}</CHAR></TEXT></P>
            <P ParaShape="1"><TEXT CharShape="4"><CHAR>{{md2hwp:list.ordered}}</CHAR></TEXT></P>
            <P ParaShape="0"><TEXT CharShape="8"><CHAR>unrelated italic paragraph</CHAR></TEXT></P>
            </SECTION></BODY></HWPML>
            """);
        var before = source.ToString();
        var result = TemplateListPrototype.FreezeMarkerFormatting(source);
        Check(source.ToString() == before, "Marker formatting modified the source template.");
        Check(result.Descendants("BULLET").First().Element("PARAHEAD")!.Attribute("CharShape") is null,
            "An unrelated bullet definition was changed.");
        Check((string?)result.Descendants("BULLET").Last().Element("PARAHEAD")!.Attribute("CharShape") == "4",
            "Implicit bullet formatting did not resolve to the sample.");
        var heads = result.Descendants("NUMBERING").Last().Elements("PARAHEAD").ToArray();
        Check((string?)heads[0].Attribute("CharShape") == "4" && (string?)heads[1].Attribute("CharShape") == "8",
            "Number marker formatting lost the sample fallback or explicit level formatting.");
        var paragraphs = result.Descendants("SECTION").Elements("P").ToArray();
        Check((string?)paragraphs[2].Attribute("ParaShape") == "0", "Unrelated paragraph formatting was changed.");
        Check((string?)paragraphs[0].Attribute("ParaShape") != "0" && (string?)paragraphs[1].Attribute("ParaShape") != "1",
            "List samples did not receive private paragraph definitions.");
        Check((int?)result.Descendants("PARASHAPELIST").Single().Attribute("Count") == 4,
            "Native definition counts were not updated.");
        Check(XNode.DeepEquals(result, TemplateListPrototype.FreezeMarkerFormatting(result)),
            "Resolved marker formatting was not idempotent.");
        var formats = XDocument.Parse("""
            <HWPML><HEAD><BORDERFILLLIST>
            <BORDERFILL Id="1"><FILLBRUSH><WINDOWBRUSH Alpha="0" FaceColor="4294967295" HatchColor="4278190080"/></FILLBRUSH></BORDERFILL>
            <BORDERFILL Id="2"/></BORDERFILLLIST><CHARSHAPELIST>
            <CHARSHAPE Id="4" Height="1000" BorderFillId="1"/>
            <CHARSHAPE Id="44" Height="1000" BorderFillId="2"/>
            </CHARSHAPELIST></HEAD></HWPML>
            """);
        var prototype = new TemplateListPrototype("bullet", 0,
            XElement.Parse("<BULLET Id='1' Char='●'><PARAHEAD CharShape='4'/></BULLET>"));
        var saved = XElement.Parse("<BULLET Id='2' Char='●'><PARAHEAD CharShape='44'/></BULLET>");
        var marker = new PreviewListMarker(1, "bullet", 0, 1, 1, true);
        prototype.Verify(saved, marker, formats);
        formats.Descendants("CHARSHAPE").Last().Add(new XElement("ITALIC"));
        var rejected = false;
        try { prototype.Verify(saved, marker, formats); }
        catch (InvalidOperationException) { rejected = true; }
        Check(rejected, "A genuinely italicized list marker escaped verification.");
        formats.Descendants("ITALIC").Remove();
        formats.Descendants("WINDOWBRUSH").Single().SetAttributeValue("FaceColor", "65535");
        rejected = false;
        try { prototype.Verify(saved, marker, formats); }
        catch (InvalidOperationException) { rejected = true; }
        Check(rejected, "A real marker background change escaped verification.");
        Console.WriteLine("Implicit list marker formatting follows its sample; explicit levels and unrelated definitions stay intact.");
    }
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
}
