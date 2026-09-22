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
TaggedBindingChecks.Run(Path.Combine(AppContext.BaseDirectory, "Fixtures", "explicit-ranges.txt"));

var borderBefore = XDocument.Parse("<HWPML><BORDERFILL Id='7'><LEFTBORDER Width='0.1' Color='0'/></BORDERFILL></HWPML>");
var borderAfter = XDocument.Parse("<HWPML><BORDERFILL Id='6'><LEFTBORDER Width='0.1' Color='0'/></BORDERFILL></HWPML>");
XElement[] borderRootBefore = [XElement.Parse("<P><CELL BorderFill='7'/></P>")];
XElement[] borderRootAfter = [XElement.Parse("<P><CELL BorderFill='6'/></P>")];
if (!TemplateRangeStructure.Equivalent(borderRootBefore, borderRootAfter, borderBefore, borderAfter))
    throw new Exception("Equivalent dereferenced border fills rejected.");
borderAfter.Descendants("LEFTBORDER").Single().SetAttributeValue("Color", "255");
if (TemplateRangeStructure.Equivalent(borderRootBefore, borderRootAfter, borderBefore, borderAfter))
    throw new Exception("Changed border-fill appearance accepted.");
Console.WriteLine("Border-fill checks passed: reference renumbering accepted, changed appearance rejected.");

var styleBefore = XDocument.Parse("<HWPML><STYLE Id='0' CharShape='0' ParaShape='0'/><CHARSHAPE Id='0' Height='1000'/><PARASHAPE Id='0'/><PARASHAPE Id='1' Align='Left'/><PARASHAPE Id='2'/></HWPML>");
var styleAfter = new XDocument(styleBefore);
styleAfter.Descendants("PARASHAPE").Single(e => (string?)e.Attribute("Id") == "2").Remove();
XElement[] staticRoots = [XElement.Parse("<P ParaShape='1'/>")];
TemplateRangeStructure.RequireOriginalStyleDefinitions(styleBefore, styleAfter, staticRoots);
foreach (var name in new[] { "STYLE", "CHARSHAPE", "PARASHAPE" })
{
    var changed = new XDocument(styleAfter);
    changed.Descendants(name).Last().SetAttributeValue("Changed", "true");
    var failed = false;
    try { TemplateRangeStructure.RequireOriginalStyleDefinitions(styleBefore, changed, staticRoots); }
    catch (InvalidOperationException) { failed = true; }
    if (!failed) throw new Exception($"Changed preserved {name} accepted.");
}
Console.WriteLine("Style preservation checks passed: unused sample shape removal accepted, referenced definition changes rejected.");

foreach (var invalidText in new[] { "", "two words", "a\nb", "a\tb", "a\u007fb" })
{
    using var json = System.Text.Json.JsonDocument.Parse(System.Text.Json.JsonSerializer.Serialize(
        new[] { new { type = "text", value = invalidText } }));
    var failed = false;
    try { InlineText.Read(json.RootElement, "/inlines"); }
    catch (InvalidDataException) { failed = true; }
    if (!failed) throw new Exception("Invalid IR text token accepted.");
}
Console.WriteLine("IR text-token checks passed: empty values, ASCII spaces, and controls rejected.");

var repository = new DirectoryInfo(AppContext.BaseDirectory);
while (repository is not null && !File.Exists(Path.Combine(repository.FullName, "dependencies", "lock.json")))
    repository = repository.Parent;
if (repository is null) throw new Exception("Repository root not found.");
var profile = InvestigationTemplateProfile.Load(Path.Combine(repository.FullName,
    "profiles", "templates", "auri-basic", "investigation-v0.1.json"), repository.FullName);
var sourcesPlan = IrPreviewPlan.Load(Path.Combine(repository.FullName,
    "examples", "commonmark-sources-v0.2.expected.ir.json"), repository.FullName, profile);
var boxes = sourcesPlan.Operations.Where(op => op.Kind == "box").ToArray();
if (boxes.Length != 2 || boxes[0].SourceRuns is null || boxes[1].SourceRuns is not null ||
    !boxes[0].SourceRuns!.Any(run => run.Emphasis) || sourcesPlan.Summary.FigureOperations != 2 ||
    string.Concat(boxes[0].SourceRuns!.Select(run => run.Text)) != "현장 조사 2026")
    throw new Exception("IR 0.2 source plan lost metadata or marks.");
foreach (var version in new[] { "0.1", "0.2" })
{
    using var sourceJson = System.Text.Json.JsonDocument.Parse(
        "{\"type\":\"verbatim_block\",\"lines\":[\"text\"],\"source\":[]}");
    var failed = false;
    try { new PlanBuilder("unused.json", repository.FullName, profile, version).AddBlock(sourceJson.RootElement, "/blocks/0"); }
    catch (InvalidDataException) { failed = true; }
    if (!failed) throw new Exception("Invalid box source accepted for " + version);
}
Console.WriteLine("Source plan checks passed: figure/box metadata and marks retained; 0.1 extension and empty 0.2 source rejected.");
