using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TableWidthLimitTests
{
    internal static void Run()
    {
        var document = XDocument.Parse("<HWPML><BODY><SECTION><P><TEXT><SECDEF><PAGEDEF Width='10000' Height='20000' GutterType='LeftRight'><PAGEMARGIN Left='1000' Right='1000' Gutter='0'/></PAGEDEF></SECDEF><COLDEF Count='1' SameSize='true' SameGap='0'/></TEXT></P><P><TEXT><TABLE><SHAPEOBJECT InstId='generated'><SIZE Width='7000'/></SHAPEOBJECT></TABLE></TEXT></P></SECTION></BODY></HWPML>");
        TableWidthLimits.RequireFits(document, ["generated"]);
        var column = document.Descendants("COLDEF").Single();
        column.SetAttributeValue("Count", "2"); column.SetAttributeValue("SameGap", "1000");
        Reject(() => TableWidthLimits.RequireFits(document, ["generated"]));
        document.Descendants("SIZE").Single().SetAttributeValue("Width", "3500");
        TableWidthLimits.RequireFits(document, ["generated"]);
        column.SetAttributeValue("SameSize", "false");
        Reject(() => TableWidthLimits.RequireFits(document, ["generated"]));
        TableWidthLimits.RequireFits(document, ["other"]);
        column.SetAttributeValue("SameSize", "true");
        document.Descendants("SECDEF").Remove();
        Reject(() => TableWidthLimits.RequireFits(document, ["generated"]));
        Console.WriteLine("Generated table widths respect active page margins and equal-width document columns.");
    }
    private static void Reject(Action action)
    {
        try { action(); } catch (InvalidDataException) { return; }
        throw new Exception("Expected a generated table width rejection.");
    }
}
