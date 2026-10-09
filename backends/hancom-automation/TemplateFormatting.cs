using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Comparison-only formatting policy. Never import these expanded references into
// Hancom. Keep this separate so future paragraph transformations can change the
// preservation policy without weakening content or object-structure checks.
internal static class TemplateFormatting
{
    public static XElement Copy(XElement source, XDocument? document = null)
    {
        // Single copies are also used while importing definitions into a mutable
        // destination. Building an index for every such lookup costs more than
        // direct resolution; use ComparisonReader explicitly for repeated checks.
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

    // One immutable document per comparison. Never retain this reader across
    // attachment/import mutations or share mutable expanded XElement instances.
    internal sealed class ComparisonReader(XDocument? document)
    {
        private Dictionary<(XName Table, string Id), XElement[]>? definitions;
        private readonly Dictionary<(XName Table, string Id), string> expanded = new();

        public XElement Copy(XElement source)
        {
            if (document is null && source.Document is { } attachedDocument)
                return new ComparisonReader(attachedDocument).Copy(source);
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
                definitions ??= document.Descendants()
                    .Where(e => e.Name == "PARASHAPE" || e.Name == "CHARSHAPE" || e.Name == "BORDERFILL" ||
                        e.Name == "TABDEF" || e.Name == "BULLET" || e.Name == "NUMBERING")
                    .Where(e => e.Attribute("Id") is not null)
                    .GroupBy(e => (e.Name, (string)e.Attribute("Id")!))
                    .ToDictionary(group => group.Key, group => group.ToArray());
                var key = ((XName)table, attribute.Value);
                if (expanded.TryGetValue(key, out var value))
                {
                    attribute.Value = value;
                    continue;
                }
                var definition = definitions.TryGetValue(key, out var matches) ? matches.SingleOrDefault() : null;
                if (definition is null)
                {
                    if (attribute.Value == "0" && table == "BORDERFILL") continue;
                    throw new InvalidOperationException($"Unknown {table} reference {attribute.Value}.");
                }
                var resolved = Copy(definition);
                resolved.Attribute("Id")?.Remove();
                value = resolved.ToString(SaveOptions.DisableFormatting);
                expanded.Add(key, value);
                attribute.Value = value;
            }
            return copy;
        }
    }
}
