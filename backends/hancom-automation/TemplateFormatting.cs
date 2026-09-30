using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Comparison-only formatting policy. Never import these expanded references into
// Hancom. Keep this separate so future paragraph transformations can change the
// preservation policy without weakening content or object-structure checks.
internal static class TemplateFormatting
{
    public static XElement Copy(XElement source, XDocument? document = null)
    {
        document ??= source.Document;
        var copy = new XElement(source);
        foreach (var element in copy.DescendantsAndSelf())
        foreach (var attribute in element.Attributes().ToArray())
        {
            var table = attribute.Name.LocalName switch
            {
                "ParaShape" => "PARASHAPE",
                "CharShape" => "CHARSHAPE",
                "BorderFill" or "BorferFill" or "BorderFillId" => "BORDERFILL",
                "TabDef" => "TABDEF",
                "Heading" when element.Name.LocalName == "PARASHAPE" =>
                    (string?)element.Attribute("HeadingType") == "Bullet" ? "BULLET" : "NUMBERING",
                _ => null,
            };
            if (table is null || attribute.Value.StartsWith("<", StringComparison.Ordinal)) continue;
            if (document is null)
                throw new InvalidOperationException($"Missing document for {attribute.Name} formatting comparison.");
            var definition = document.Descendants(table).SingleOrDefault(e => (string?)e.Attribute("Id") == attribute.Value);
            if (definition is null)
            {
                if (attribute.Value == "0" && table == "BORDERFILL") continue;
                throw new InvalidOperationException($"Unknown {table} reference {attribute.Value}.");
            }
            var resolved = Copy(definition, document);
            resolved.Attribute("Id")?.Remove();
            attribute.Value = resolved.ToString(SaveOptions.DisableFormatting);
        }
        return copy;
    }
}
