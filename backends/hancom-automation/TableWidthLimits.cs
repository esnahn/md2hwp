using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Validate the generated anchor against the section/column definitions which
// actually precede it, after heading blocks and retained controls are attached.
internal static class TableWidthLimits
{
    internal static void RequireFits(XDocument document, IReadOnlyCollection<string> instances)
    {
        var generated = instances.ToHashSet(StringComparer.Ordinal);
        if (generated.Count == 0) return;
        foreach (var section in document.Descendants("SECTION"))
        {
            XElement? page = null;
            XElement? columns = null;
            foreach (var paragraph in section.Elements("P"))
            foreach (var control in paragraph.Descendants().Where(e => !e.Ancestors().Any(a =>
                         a.Name.LocalName is "HEADER" or "FOOTER" or "MASTERPAGE")))
            {
                if (control.Name.LocalName == "SECDEF" && control.Element("PAGEDEF") is { } definition)
                    page = definition;
                else if (control.Name.LocalName == "COLDEF") columns = control;
                else if (control.Name.LocalName == "TABLE" &&
                         generated.Contains((string?)control.Element("SHAPEOBJECT")?.Attribute("InstId") ?? ""))
                {
                    if (page is null) throw new InvalidDataException("Generated table has no active section page definition.");
                    var margin = page.Element("PAGEMARGIN") ?? throw new InvalidDataException("Missing table page margins.");
                    var usable = Integer(page, "Width") - Integer(margin, "Left") - Integer(margin, "Right");
                    if ((string?)page.Attribute("GutterType") != "TopBottom") usable -= Integer(margin, "Gutter");
                    var count = columns is null ? 1 : Integer(columns, "Count");
                    if (count < 1) throw new InvalidDataException("Invalid active table column count.");
                    if (count > 1)
                    {
                        if ((string?)columns!.Attribute("SameSize") != "true")
                            throw new InvalidDataException("Generated tables currently require equal-width document columns. Use a single or equal-width column layout.");
                        usable = (usable - Integer(columns, "SameGap") * (count - 1)) / count;
                    }
                    var size = control.Element("SHAPEOBJECT")?.Element("SIZE")
                        ?? throw new InvalidDataException("Generated table has no native size.");
                    var width = Integer(size, "Width");
                    if (width <= 0 || width > usable)
                        throw new InvalidDataException($"table.width-mm exceeds the active text column: table={width * 25.4 / 7200:0.###} mm, available={usable * 25.4 / 7200:0.###} mm. Reduce the table width in the template.");
                }
            }
        }
    }

    private static int Integer(XElement element, string name) =>
        int.TryParse((string?)element.Attribute(name), out var value) && value >= 0
            ? value : throw new InvalidDataException($"Missing or invalid table layout attribute {element.Name.LocalName}/@{name}.");
}
