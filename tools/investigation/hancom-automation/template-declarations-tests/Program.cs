using Md2Hwp.HancomIrPreview;
using System.Xml.Linq;

var fixture = File.ReadAllLines(Path.Combine(AppContext.BaseDirectory, "Fixtures", "explicit-ranges.txt"));
var plan = TemplateDeclarations.Parse(fixture);
if (plan.ContentParagraph != 1 || plan.SamplesBegin != 3 || plan.SamplesEnd != 21 ||
    plan.Styles["body"] != 5 || plan.Styles["heading.1"] != 6 || plan.Prototypes.Count != 4 ||
    plan.Prototypes["figure"].StartInclusive != 11 || plan.Prototypes["figure"].EndExclusive != 14 ||
    plan.Prototypes["figure"].Slots["figure.caption"] != 12)
    throw new Exception("Explicit ranges or slots do not match the fixture.");

var rejected = 0;
void Reject(string name, Func<List<string>, List<string>> mutate, string expected)
{
    try { TemplateDeclarations.Parse(mutate(fixture.ToList())); }
    catch (InvalidDataException error)
    {
        if (!error.Message.Contains("paragraph[", StringComparison.Ordinal) ||
            !error.Message.Contains(expected, StringComparison.Ordinal))
            throw new Exception($"{name}: incorrect diagnostic: {error.Message}");
        rejected++;
        return;
    }
    throw new Exception($"{name}: unexpectedly accepted");
}
List<string> Replace(List<string> lines, int index, string value) { lines[index] = value; return lines; }
Reject("crossed range", x => Replace(x, 9, "{{md2hwp:end:figure}}"), "crossed");
Reject("unclosed range", x => { x.RemoveAt(21); return x; }, "unclosed");
Reject("missing begin", x => { x.RemoveAt(10); return x; }, "orphan");
Reject("nested prototype", x => Replace(x, 8, "{{md2hwp:begin:figure}}"), "nested");
Reject("duplicate content", x => Replace(x, 0, "{{md2hwp:content}}"), "duplicate");
Reject("content in samples", x => Replace(x, 6, "{{md2hwp:content}}"), "scoped");
Reject("duplicate style", x => Replace(x, 6, "{{md2hwp:body}}"), "duplicate");
Reject("wrong owner", x => Replace(x, 8, "{{md2hwp:slot:figure.image}}"), "wrong-owner");
Reject("missing slot", x => { x.RemoveAt(12); return x; }, "missing slot");
Reject("duplicate slot", x => { x.Insert(12, x[11]); return x; }, "duplicate slot");
Reject("order", x => { (x[11], x[12]) = (x[12], x[11]); return x; }, "out of order");
Reject("unknown", x => Replace(x, 6, "{{md2hwp:heading.7}}"), "unknown");
Reject("whitespace", x => Replace(x, 5, " {{md2hwp:body}}"), "whole paragraph");
Reject("inline", x => Replace(x, 5, "text {{md2hwp:body}}"), "whole paragraph");
Reject("two tokens", x => Replace(x, 5, "{{md2hwp:body}}{{md2hwp:heading.1}}"), "unknown");
Reject("newline", x => Replace(x, 5, "{{md2hwp:body}}\n"), "whole paragraph");
Reject("empty prototype", x => { x.RemoveAt(8); return x; }, "empty prototype");
Reject("missing version", x => { x.RemoveAt(4); return x; }, "missing required");
Reject("unknown version", x => Replace(x, 4, "{{md2hwp:contract:experimental-2}}"), "unknown");
Reject("duplicate version", x => Replace(x, 6, x[4]), "duplicate");
Reject("outside samples", x => Replace(x, 0, "{{md2hwp:body}}"), "outside samples");
Reject("duplicate samples", x => Replace(x, 22, "{{md2hwp:begin:samples}}"), "duplicate samples");
Reject("duplicate prototype", x => { x.InsertRange(10, x.GetRange(7, 3)); return x; }, "duplicate");
Reject("missing successor", x => { x.RemoveAt(22); return x; }, "successor");

// Content may be before or after samples; it never belongs to a prototype.
var after = fixture.Where((_, i) => i != 1).ToList();
after.Add("{{md2hwp:content}}");
after.Add("terminal");
TemplateDeclarations.Parse(after);
// No scanner is invoked on manuscript content; tokens inserted from IR remain literal.
var minimal = new[] { "{{md2hwp:content}}", "{{md2hwp:begin:samples}}",
    "{{md2hwp:contract:experimental-1}}", "{{md2hwp:body}}", "{{md2hwp:end:samples}}", "" };
TemplateDeclarations.Parse(minimal);
Console.WriteLine($"Template declaration lexical checks passed: 3 accepted, {rejected} rejected. No COM or HWP validation performed.");

XElement[] Shapes(int first, int second, string text = "unchanged") =>
    [XElement.Parse($"<P Style='1'><SHAPEOBJECT ZOrder='{first}'/><CHAR>{text}</CHAR></P>"),
     XElement.Parse($"<P Style='2'><SHAPEOBJECT ZOrder='{second}'/></P>")];
if (!TemplateRangeStructure.Equivalent(Shapes(35, 32), Shapes(5, 2)) ||
    TemplateRangeStructure.Equivalent(Shapes(35, 32), Shapes(2, 5)) ||
    TemplateRangeStructure.Equivalent(Shapes(35, 35), Shapes(5, 2)) ||
    TemplateRangeStructure.Equivalent(Shapes(35, 32), Shapes(5, 2, "changed")))
    throw new Exception("ZOrder renumbering must preserve ordering, ties, and all other content.");
var changedStyle = Shapes(5, 2);
changedStyle[0].SetAttributeValue("Style", "9");
if (TemplateRangeStructure.Equivalent(Shapes(35, 32), changedStyle))
    throw new Exception("Structural comparison accepted a changed paragraph style.");
Console.WriteLine("Structure checks passed: renumbering accepted; order, ties, text, and style changes rejected.");
