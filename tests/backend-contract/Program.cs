using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
static void Reject(Action operation) {
    try { operation(); } catch (InvalidDataException) { return; }
    throw new Exception("Expected rejection.");
}
var metadata = new Dictionary<string,string> {
    ["title"]="문서 제목", ["date"]="2026년 1월 2일", ["date-meta"]="2026-01-02",
    ["md2hwp-report-number"]="기본 2026-01", ["md2hwp-literal"]="{{md2hwp:meta:title}}"
};
var input = XDocument.Parse("<DOC><P Style='1'><TEXT CharShape='4'><CHAR>표지 {{md2hwp:me</CHAR></TEXT><TEXT CharShape='5'><CHAR>ta:title}} / {{md2hwp:meta:date-meta:fmt:%Y년 %-m월 %-d일}} 끝</CHAR></TEXT></P><P><TEXT><TABLE><P><TEXT CharShape='6'><CHAR>{{md2hwp:meta:md2hwp-report-number}}</CHAR></TEXT></P></TABLE><CHAR>{{md2hwp:meta:md2hwp-literal}}</CHAR></TEXT></P></DOC>");
var splitOnly = XDocument.Parse("<DOC><P><TEXT CharShape='4'><CHAR>{{md2hwp:meta:</CHAR></TEXT><TEXT CharShape='5'><CHAR>title}}</CHAR></TEXT></P></DOC>");
var splitResult = TemplateMetadata.Apply(splitOnly,metadata);
Check(splitResult.Descendants("TEXT").Count()==1 && splitResult.Descendants("CHAR").Single().Value=="문서 제목","Consumed empty formatting run retained.");
var original = input.ToString();
var prepared = TemplateMetadata.Prepare(input,metadata);
Check(!prepared.Document.ToString().Contains("{{md2hwp:"),"Literal metadata reached template binding.");
Check(XNode.DeepEquals(prepared.Restore(prepared.Document),TemplateMetadata.Apply(input,metadata)),"Deferred restoration differs.");
var output = TemplateMetadata.Apply(input,metadata);
Check(input.ToString()==original,"Source mutated.");
var first = output.Descendants("P").First();
Check(TaggedTemplateBinding.DirectText(first)=="표지 문서 제목 / 2026년 1월 2일 끝","Split-run replacement.");
Check(first.Elements("TEXT").First().Element("CHAR")!.Value=="표지 문서 제목","First-run formatting lost.");
Check(output.Descendants("TABLE").Descendants("CHAR").Single().Value=="기본 2026-01","Nested cell replacement.");
Check(output.Descendants("P").Skip(1).First().Element("TEXT")!.Elements("CHAR").Single().Value=="{{md2hwp:meta:title}}","Replacement recursively interpreted.");
Check(TemplateMetadata.FormatDate("2026-01-02","%Y/%m/%d %y %-m %-d %F %%")=="2026/01/02 26 1 2 2026-01-02 %","fmt output.");
foreach (var fmt in new[]{"", "%", "%Q", "%H", "%-Y", "%n", "line\n"})
    Reject(()=>TemplateMetadata.FormatDate("2026-01-02",fmt));
Reject(()=>TemplateMetadata.Apply(input,new Dictionary<string,string>()));
Reject(()=>TemplateMetadata.Apply(XDocument.Parse("<DOC><P><TEXT><CHAR>{{md2hwp:meta:title:fmt:%Y}}</CHAR></TEXT></P></DOC>"),metadata));
Reject(()=>TemplateMetadata.Apply(XDocument.Parse("<DOC><P><TEXT><CHAR>{{md2hwp:meta:ti</CHAR><AUTONUM/><CHAR>tle}}</CHAR></TEXT></P></DOC>"),metadata));
using var valid = JsonDocument.Parse("{\"author\":[\"홍길동\",\"김연구\"],\"date\":\"미정\",\"md2hwp-test\":\"값\"}");
Check(TemplateMetadata.Read(valid.RootElement)["author"]=="홍길동, 김연구","Author joining.");
foreach(var invalid in new[]{"{\"lang\":\"ko-KR\"}","{\"date-meta\":\"2026-02-29\",\"date\":\"2026년 2월 29일\"}","{\"author\":[]}","{\"author\":[\"\",\"\"]}"}) {
    using var value=JsonDocument.Parse(invalid); Reject(()=>TemplateMetadata.Read(value.RootElement));
}
Console.WriteLine("Backend metadata/fmt contract checks passed.");

if (args.Length==2) {
    var expected=HancomPreviewWriter.NormalizeFigureMatrices(HwpMarkup.Parse(File.ReadAllText(args[0])));
    var actual=HancomPreviewWriter.NormalizeFigureMatrices(HwpMarkup.Parse(File.ReadAllText(args[1])));
    var before=AuriMinimalBoxPrototype.RootParagraphs(expected);
    var after=AuriMinimalBoxPrototype.RootParagraphs(actual);
    TemplateRangeStructure.RequireOriginalStyleDefinitions(expected,actual,before);
    Check(TemplateRangeStructure.Equivalent(before,after,expected,actual),"Native template changed: "+TemplateRangeStructure.DescribeDifference(before,after));
    Console.WriteLine("Native template save/reopen structure and styles preserved.");
}
