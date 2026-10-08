using System.Xml.Linq;
using System.Text.RegularExpressions;

namespace Md2Hwp.HancomIrPreview;

internal static class TemplateHeadingEach
{
    private static readonly Regex Marker = new(@"^\{\{md2hwp:(begin|end):each\.child:heading([1-6])\}\}$", RegexOptions.CultureInvariant);

    internal static XElement[] Validate(IEnumerable<XElement> roots, int level)
    {
        var slots = new List<XElement>();
        Rewrite(roots, level, null, null, (p, _, _) => slots.Add(p), 0);
        return slots.ToArray();
    }

    internal static XElement[] Expand(IEnumerable<XElement> roots, int level, int index, IrPreviewPlan plan,
        Action<XElement, PreviewOperation, int> fill, HeadingReferenceNumbers.Target?[]? counted = null) => Rewrite(roots, level, index, plan,
            (p, operation, ordinal) => fill(p, operation!, ordinal), 0, numbers: counted ?? HeadingReferenceNumbers.CountOperations(plan.Operations));

    private static XElement[] Rewrite(IEnumerable<XElement> input, int level, int? index, IrPreviewPlan? plan,
        Action<XElement, PreviewOperation?, int> fill, int ordinal, bool requireSlot = true, Action<int>? reportCount = null, HeadingReferenceNumbers.Target?[]? numbers = null)
    {
        var paragraphs = input.Select(p => new XElement(p)).ToArray();
        var output = new List<XElement>();
        var slots = 0;
        for (var i = 0; i < paragraphs.Length; i++)
        {
            var p = paragraphs[i];
            var text = TaggedTemplateBinding.DirectText(p);
            var marker = Marker.Match(text);
            if (marker.Success)
            {
                Plain(p);
                if (marker.Groups[1].Value != "begin") throw new InvalidDataException("Unmatched each.child end boundary.");
                var childLevel = int.Parse(marker.Groups[2].Value);
                if (childLevel != level + 1) throw new InvalidDataException($"heading{level} can repeat only each.child:heading{level + 1}.");
                var stack = new Stack<int>(); stack.Push(childLevel);
                int end = i + 1;
                for (; end < paragraphs.Length; end++)
                {
                    var nested = Marker.Match(TaggedTemplateBinding.DirectText(paragraphs[end]));
                    if (!nested.Success) continue;
                    Plain(paragraphs[end]);
                    var n = int.Parse(nested.Groups[2].Value);
                    if (nested.Groups[1].Value == "begin") stack.Push(n);
                    else if (stack.Count == 0 || stack.Pop() != n) throw new InvalidDataException("Mismatched each.child boundaries.");
                    if (stack.Count == 0) break;
                }
                if (end == paragraphs.Length) throw new InvalidDataException("each.child boundaries must pair within one paragraph container (same body, cell or text box).");
                var sample = paragraphs.Skip(i + 1).Take(end - i - 1).ToArray();
                if (plan is null)
                    Rewrite(sample, childLevel, null, null, (_, _, _) => { }, 0);
                else
                {
                    var number = 0;
                    foreach (var child in Children(plan, index!.Value, level))
                    {
                        var copies = Rewrite(sample, childLevel, child, plan, fill, ++number, numbers: numbers);
                        if ((string?)p.Attribute("PageBreak") == "true" && copies.Length > 0)
                        { copies[0].SetAttributeValue("PageBreak", "true"); copies[0].SetAttributeValue("ColumnBreak", "false"); }
                        output.AddRange(copies);
                    }
                }
                if ((string?)paragraphs[end].Attribute("PageBreak") == "true" && end + 1 < paragraphs.Length)
                { paragraphs[end + 1].SetAttributeValue("PageBreak", "true"); paragraphs[end + 1].SetAttributeValue("ColumnBreak", "false"); }
                i = end; continue;
            }
            void Visit(XElement element)
            {
                if (element.Name.LocalName == "P")
                {
                    var direct = TaggedTemplateBinding.DirectText(element);
                    var clean = TemplateHeadingNumbers.WithoutNumbers(direct, level);
                    if (plan is not null)
                        TemplateHeadingNumbers.FillParagraph(element, level, numbers![index!.Value]!.Numbers);
                    var slot = TaggedTemplateBinding.Tag($"slot:heading{level}");
                    if (clean.Contains(slot, StringComparison.Ordinal))
                    {
                        Plain(element);
                        if (clean.Split(slot, StringSplitOptions.None).Length != 2 || clean.Replace(slot, "", StringComparison.Ordinal).Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
                            throw new InvalidDataException($"heading{level} paragraph requires one title slot and only own/ancestor number tags.");
                        slots++; fill(element, plan is null ? null : plan.Operations[index!.Value], ordinal); return;
                    }
                    if (clean.Contains(TaggedTemplateBinding.Prefix, StringComparison.Ordinal))
                        throw new InvalidDataException($"Unexpected declaration in heading{level} scope: {direct}");
                }
                if (element.Elements("P").Any())
                {
                    // A nested paragraph container belongs to the same heading.
                    // It need not itself contain a title, so count slots in the caller.
                    var result = RewriteContainer(element.Elements("P"), level, index, plan, fill, ordinal, numbers, out var count);
                    slots += count;
                    if (result.Length == 0)
                    {
                        // Hancom paragraph containers require a terminal empty
                        // paragraph even when the entire repeated range is empty.
                        var blank = new XElement(element.Elements("P").First());
                        blank.Attribute("PageBreak")?.Remove(); blank.Attribute("ColumnBreak")?.Remove();
                        var charShape = (string?)blank.Element("TEXT")?.Attribute("CharShape") ?? "0";
                        blank.ReplaceNodes(new XElement("TEXT", new XAttribute("CharShape", charShape)));
                        result = [blank];
                    }
                    element.ReplaceNodes(result); return;
                }
                foreach (var child in element.Elements().ToArray()) Visit(child);
            }
            Visit(p); output.Add(p);
        }
        reportCount?.Invoke(slots);
        if (requireSlot && slots == 0) throw new InvalidDataException($"heading{level} scope requires at least one slot:heading{level} paragraph.");
        return output.ToArray();
    }

    private static XElement[] RewriteContainer(IEnumerable<XElement> input, int level, int? index, IrPreviewPlan? plan,
        Action<XElement, PreviewOperation?, int> fill, int ordinal, HeadingReferenceNumbers.Target?[]? numbers, out int count)
    {
        var found = 0;
        var result = Rewrite(input, level, index, plan, fill, ordinal, false, n => found = n, numbers);
        count = found; return result;
    }

    private static IEnumerable<int> Children(IrPreviewPlan plan, int index, int level)
    {
        for (var i = index + 1; i < plan.Operations.Count; i++)
        {
            var role = plan.Operations[i].ParagraphStyle;
            if (plan.Operations[i].Kind != "text" || role is null || !role.StartsWith("heading", StringComparison.Ordinal)) continue;
            var next = int.Parse(role[7..]);
            if (next <= level) yield break;
            if (next == level + 1) yield return i;
        }
    }

    private static void Plain(XElement p)
    {
        if (p.Elements().Any(e => e.Name.LocalName != "TEXT") || p.Elements().SelectMany(e => e.Elements()).Any(e => e.Name.LocalName != "CHAR" || e.HasElements))
            throw new InvalidDataException("each.child boundaries must be standalone; heading slots must be plain-text paragraphs without native controls.");
    }
}
