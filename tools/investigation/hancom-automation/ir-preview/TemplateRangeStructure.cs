using System.Globalization;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal static class TemplateRangeStructure
{
    // Hancom renumbers ZOrder on save. Preserve its complete relative order,
    // including ties; compare every other XML node and attribute exactly.
    public static bool Equivalent(IReadOnlyList<XElement> before, IReadOnlyList<XElement> after)
    {
        if (before.Count != after.Count) return false;
        var left = Canonicalize(before);
        var right = Canonicalize(after);
        return left.Zip(right).All(pair => XNode.DeepEquals(pair.First, pair.Second));
    }

    private static XElement[] Canonicalize(IReadOnlyList<XElement> roots)
    {
        var copies = roots.Select(root => new XElement(root)).ToArray();
        var orders = copies.SelectMany(root => root.DescendantsAndSelf())
            .Where(element => element.Name.LocalName == "SHAPEOBJECT")
            .Select(element => element.Attribute("ZOrder")).OfType<XAttribute>().ToArray();
        var values = orders.Select(order => int.Parse(order.Value, CultureInfo.InvariantCulture)).ToArray();
        var ranks = values.Distinct().Order().Select((value, rank) => (value, rank))
            .ToDictionary(pair => pair.value, pair => pair.rank);
        for (var i = 0; i < orders.Length; i++)
            orders[i].Value = ranks[values[i]].ToString(CultureInfo.InvariantCulture);
        return copies;
    }
}
