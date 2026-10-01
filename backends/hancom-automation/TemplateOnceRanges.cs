using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Inline ranges are local to one paragraph, including paragraphs inside objects.
internal static class TemplateOnceRanges
{
    private sealed record Atom(XElement Node, char? Character);

    internal static void Apply(IEnumerable<XElement> roots, bool first)
    {
        foreach (var paragraph in roots.SelectMany(p => p.DescendantsAndSelf("P")).ToArray())
        {
            var atoms = paragraph.Elements("TEXT").SelectMany(t => t.Elements()).SelectMany(e =>
                e.Name.LocalName == "CHAR" && !e.HasElements
                    ? e.Value.Select(c => new Atom(e, c)) : new[] { new Atom(e, null) }).ToArray();
            var keep = Enumerable.Repeat(true, atoms.Length).ToArray();
            var inside = false;
            var found = false;
            bool Matches(int start, string token) => start + token.Length <= atoms.Length &&
                token.Select((c, j) => atoms[start + j].Character == c).All(equal => equal);
            for (var i = 0; i < atoms.Length;)
            {
                var begin = Matches(i, TaggedTemplateBinding.Tag("begin:once"));
                var end = Matches(i, TaggedTemplateBinding.Tag("end:once"));
                if (begin || end)
                {
                    if (begin == inside) throw new InvalidDataException("once ranges must be paired in one paragraph and cannot be nested.");
                    var length = TaggedTemplateBinding.Tag(begin ? "begin:once" : "end:once").Length;
                    Array.Fill(keep, false, i, length);
                    inside = begin; found = true; i += length; continue;
                }
                if (inside)
                {
                    if (Matches(i, TaggedTemplateBinding.Prefix) ||
                        (atoms[i].Character is null && atoms[i].Node.Descendants("CHAR").Any(c => c.Value.Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))))
                        throw new InvalidDataException("once ranges cannot contain template declarations, title slots or nested once ranges.");
                    if (!first) keep[i] = false;
                }
                i++;
            }
            if (inside) throw new InvalidDataException("begin:once requires end:once in the same paragraph.");
            if (!found) continue;
            foreach (var group in atoms.Select((atom, index) => (atom, index)).GroupBy(x => x.atom.Node))
            {
                if (group.First().atom.Character is null)
                {
                    if (!keep[group.First().index]) group.Key.Remove();
                }
                else
                {
                    var text = new string(group.Where(x => keep[x.index]).Select(x => x.atom.Character!.Value).ToArray());
                    if (text.Length == 0) group.Key.Remove(); else group.Key.Value = text;
                }
            }
            var charShape = (string?)paragraph.Element("TEXT")?.Attribute("CharShape") ?? "0";
            paragraph.Elements("TEXT").Where(t => !t.HasElements).Remove();
            if (!paragraph.Elements("TEXT").Any())
                paragraph.Add(new XElement("TEXT", new XAttribute("CharShape", charShape), new XElement("CHAR", "")));
        }
    }
}
