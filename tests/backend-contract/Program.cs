using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
static void Reject(Action operation) {
    try { operation(); } catch (InvalidDataException) { return; }
    throw new Exception("Expected rejection.");
}
var sectionFixture = XDocument.Parse("<HWPML><BODY><SECTION><P Style='0' PageBreak='false' ColumnBreak='false'><TEXT CharShape='1'><COLDEF/><SECDEF><MASTERPAGE Type='Even'><PARALIST><P><TEXT><LINE/><CHAR/></TEXT></P></PARALIST></MASTERPAGE></SECDEF><CHAR>{{md2hwp:begin:template}}</CHAR></TEXT></P><P><TEXT><CHAR>{{md2hwp:ir-version:0.3}}</CHAR></TEXT></P></SECTION></BODY></HWPML>");
var originalSectionFixture = sectionFixture.ToString();
var retainedSection = new XElement(sectionFixture.Descendants("SECDEF").Single());
var separatedSection = TaggedTemplateBinding.PreserveBeginSectionSettings(sectionFixture);
var sectionRoots = AuriMinimalBoxPrototype.RootParagraphs(separatedSection);
Check(sectionFixture.ToString()==originalSectionFixture,"Section-boundary normalization mutated source.");
Check(sectionRoots.Count==3 && TaggedTemplateBinding.DirectText(sectionRoots[0])=="" && TaggedTemplateBinding.DirectText(sectionRoots[1])=="{{md2hwp:begin:template}}","Section settings not separated from the declaration.");
Check(XNode.DeepEquals(retainedSection,sectionRoots[0].Descendants("SECDEF").Single()),"Section or master-page settings changed.");
Check(!sectionRoots[1].Descendants("SECDEF").Any() && !sectionRoots[1].Descendants("P").Any(),"Section settings retained in disposable declaration.");
Check(sectionRoots[1].Attribute("PageBreak") is null && sectionRoots[1].Attribute("ColumnBreak") is null,"False break flags retained in noninitial declaration.");
Check(XNode.DeepEquals(separatedSection,TaggedTemplateBinding.PreserveBeginSectionSettings(separatedSection)),"Section normalization is not idempotent.");
foreach (var tag in new[]{"end:template", "body", "begin:heading1", "begin:figure.caption"}) {
    var other=new XDocument(sectionFixture);
    other.Descendants("CHAR").Last().Value="unchanged";
    var otherFirst=AuriMinimalBoxPrototype.RootParagraphs(other)[0];
    otherFirst.Elements("TEXT").Elements("CHAR").Single().Value=TaggedTemplateBinding.Tag(tag);
    Check(XNode.DeepEquals(other,TaggedTemplateBinding.PreserveBeginSectionSettings(other)),"Section exception leaked into another declaration.");
}
var splitSection = new XDocument(sectionFixture);
var splitFirst = AuriMinimalBoxPrototype.RootParagraphs(splitSection)[0];
splitFirst.Elements("TEXT").Elements("CHAR").Remove();
splitFirst.Add(new XElement("TEXT",new XAttribute("CharShape","2"),new XElement("CHAR","{{md2hwp:begin:")),new XElement("TEXT",new XAttribute("CharShape","3"),new XElement("CHAR","template}}")));
var splitSectionRoots=AuriMinimalBoxPrototype.RootParagraphs(TaggedTemplateBinding.PreserveBeginSectionSettings(splitSection));
Check(TaggedTemplateBinding.DirectText(splitSectionRoots[0])=="" && TaggedTemplateBinding.DirectText(splitSectionRoots[1])=="{{md2hwp:begin:template}}","Split declaration runs leaked into retained section paragraph.");
Check(splitSectionRoots[1].Elements("TEXT").All(e=>e.HasElements),"Empty control-only run retained in declaration.");
var unsupportedSection = new XDocument(sectionFixture);
AuriMinimalBoxPrototype.RootParagraphs(unsupportedSection)[0].Element("TEXT")!.Add(new XElement("TABLE"));
Check(XNode.DeepEquals(unsupportedSection,TaggedTemplateBinding.PreserveBeginSectionSettings(unsupportedSection)),"Unrelated table control accepted on begin:template.");
Console.WriteLine("Begin-template section preservation checks passed.");
static XElement HeadingParagraph(string text) => new("P", new XAttribute("Style", "0"), new XAttribute("ParaShape", "0"), new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", text)));
var headingFixture = new XDocument(new XElement("HWPML",new XElement("BODY",new XElement("SECTION",new[]{"begin:template","begin:heading1","slot:heading1","begin:each.child:heading2","slot:heading2","end:each.child:heading2","end:heading1","end:template"}.Select(t=>HeadingParagraph(TaggedTemplateBinding.Tag(t)))))));
var loweredHeading = TemplateHeadingBlocks.Lower(headingFixture);
Check(AuriMinimalBoxPrototype.RootParagraphs(loweredHeading.Document).Any(p=>TaggedTemplateBinding.DirectText(p)=="{{md2hwp:heading1}}"),"heading1 block was not lowered to the new role.");
var repeatFixture = new[]{"slot:heading1","begin:each.child:heading2","slot:heading2","end:each.child:heading2"}.Select(t=>HeadingParagraph(TaggedTemplateBinding.Tag(t))).ToArray();
var headingOperations = new[]{new PreviewOperation("text","parent",[],ParagraphStyle:"heading1"),new PreviewOperation("text","child A",[],ParagraphStyle:"heading2"),new PreviewOperation("text","nested",[],ParagraphStyle:"heading3"),new PreviewOperation("text","child B",[],ParagraphStyle:"heading2"),new PreviewOperation("text","next parent",[],ParagraphStyle:"heading1"),new PreviewOperation("text","other child",[],ParagraphStyle:"heading2")};
var headingPlan = new IrPreviewPlan("fixture","fixture",new PreviewSummary(6,6,0,0,0),headingOperations,[]);
var repeatedHeadings=TemplateHeadingEach.Expand(repeatFixture,1,0,headingPlan,(p,operation,_)=>p.Element("TEXT")!.Element("CHAR")!.Value=operation.Label);
Check(repeatedHeadings.Select(TaggedTemplateBinding.DirectText).SequenceEqual(new[]{"parent","child A","child B"}),"heading2 child repetition parsed the wrong heading level or crossed the next heading1.");
Reject(()=>TemplateHeadingEach.Validate(new[]{HeadingParagraph("{{md2hwp:slot:heading.1}}")},1));
var attachFixture=new XDocument(headingFixture);
var headingDefinitions=new XElement("HEAD",new XElement("PARASHAPELIST",new XElement("PARASHAPE",new XAttribute("Id","0"))),new XElement("CHARSHAPELIST",new XElement("CHARSHAPE",new XAttribute("Id","0"))));
attachFixture.Root!.AddFirst(headingDefinitions);
var attachLayout=TemplateHeadingBlocks.Lower(attachFixture).Layout;
var renderedHeading=new XDocument(new XElement("HWPML",new XElement(headingDefinitions),new XElement("BODY",new XElement("SECTION",HeadingParagraph("generated title")))));
var attachPlan=new IrPreviewPlan("fixture","fixture",new PreviewSummary(1,1,0,0,0),new[]{new PreviewOperation("text","title",new[]{"attached title"},ParagraphStyle:"heading1")},[]);
Check(TaggedTemplateBinding.DirectText(AuriMinimalBoxPrototype.RootParagraphs(attachLayout.Attach(renderedHeading,attachPlan,0)).Single())=="attached title","heading1 block attachment did not parse and fill its title.");
Console.WriteLine("Heading role, block attachment and child repetition checks passed.");
var startMetadata=new Dictionary<string,string>{{"md2hwp-heading1-start","3"}};
Check(TemplateHeadingNumbers.Start(startMetadata)==3 && TemplateHeadingNumbers.Start(new Dictionary<string,string>())==1,"Heading1 start setting.");
foreach(var invalidStart in new[]{"0","-1","1.5","2147483648"," 3","+3"}) Reject(()=>TemplateHeadingNumbers.Start(new Dictionary<string,string>{{"md2hwp-heading1-start",invalidStart}}));
var trackedChapters=TemplateHeadingNumbers.Track(headingOperations,3);
Check(trackedChapters.Select(o=>o.Heading1Number).SequenceEqual(new int?[]{3,3,3,3,4,4}),"Heading1 tracking incremented child headings.");
Check(TemplateHeadingNumbers.Track(new[]{new PreviewOperation("figure","figure",[])},3).Single().Heading1Number is null,"Figure before first heading1 acquired a chapter.");
Reject(()=>TemplateHeadingNumbers.Track(headingOperations,int.MaxValue));
var chapterSlots=XDocument.Parse("<DOC><P><TEXT CharShape='4'><CHAR>Chapter {{md2hwp:num:head</CHAR></TEXT><TEXT CharShape='5'><CHAR>ing1}} 끝</CHAR></TEXT></P></DOC>");
var preparedChapterSlots=TemplateHeadingNumbers.Prepare(chapterSlots);
var filledChapterSlots=TemplateHeadingNumbers.Fill(preparedChapterSlots.Root!.Elements(),3);
Check(TaggedTemplateBinding.DirectText(filledChapterSlots.Single())=="Chapter 3 끝","Split-run heading1 number slot.");
Check((string?)filledChapterSlots.Single().Element("TEXT")!.Attribute("CharShape")=="4","Heading1 number formatting changed.");
Reject(()=>TemplateHeadingNumbers.Fill(preparedChapterSlots.Root!.Elements(),null));
Reject(()=>TemplateHeadingNumbers.RequireResolved(preparedChapterSlots));
var unresolvedText=TaggedTemplateBinding.DirectText(preparedChapterSlots.Root!.Elements().Single());
var splitUnresolved=new XDocument(new XElement("DOC",new XElement("P",new XElement("TEXT",new XElement("CHAR",unresolvedText[..(unresolvedText.Length/2)])),new XElement("TEXT",new XElement("CHAR",unresolvedText[(unresolvedText.Length/2)..])))));
Reject(()=>TemplateHeadingNumbers.RequireResolved(splitUnresolved));
var nativeCaptionRoots=new List<XElement>();
foreach(var chapter in new[]{3,3,4,4}) {
    nativeCaptionRoots.Add(new XElement("P",new XElement("TEXT",new XAttribute("CharShape","0"),new XElement("PICTURE",new XElement("SHAPEOBJECT",new XElement("SIZE",new XAttribute("Width","10000")))))));
    var paragraph=HeadingParagraph("[그림 "+TemplateHeadingNumbers.Tag+"-");
    paragraph.Element("TEXT")!.Add(new XElement("AUTONUM",new XAttribute("NumberType","Figure"),new XAttribute("Number","99")),new XElement("CHAR","] caption"));
    nativeCaptionRoots.Add(paragraph);
}
var fakeCaptionDocument=TemplateHeadingNumbers.Prepare(new XDocument(new XElement("HWPML",new XElement("BODY",new XElement("SECTION",nativeCaptionRoots)))));
var captionPlan=new IrPreviewPlan("fixture","fixture",new PreviewSummary(4,0,0,4,0),new[]{3,3,4,4}.Select(n=>new PreviewOperation("figure","figure",new[]{"image","caption",""},Heading1Number:n)).ToArray(),[]);
var captionLayout=new NativeFigureCaption(new XElement("CAPTION",new XAttribute("Side","Bottom"),new XElement("PARALIST")));
var chapterPictures=captionLayout.Attach(fakeCaptionDocument,captionPlan,0);
TemplateHeadingBlocks.RecalculateFigureNumbers(chapterPictures);
TemplateHeadingNumbers.RequireResolved(chapterPictures);
Check(chapterPictures.Descendants("NEWNUM").Count()==2,"Figure counters must restart once per chapter.");
Check(chapterPictures.Descendants("AUTONUM").Select(e=>(string)e.Attribute("Number")!).SequenceEqual(new[]{"1","2","1","2"}),"Figure counters did not restart at the second chapter.");
Check(chapterPictures.Descendants("CAPTION").Select(c=>TaggedTemplateBinding.DirectText(c.Descendants("P").Single())).SequenceEqual(new[]{"[그림 3-] caption","[그림 3-] caption","[그림 4-] caption","[그림 4-] caption"}),"Caption chapter numbers were not filled from the figure operation.");
Console.WriteLine("Heading1 start, number slots and native figure restart checks passed.");
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
    _ = TaggedTemplateBinding.Read(TemplateMetadata.Prepare(actual,metadata).Document,args[1]);
    Check(expected.Descendants("BINDATA").Select(e=>e.Value).SequenceEqual(actual.Descendants("BINDATA").Select(e=>e.Value)),"Embedded template image data changed.");
    Console.WriteLine("Native template save/reopen structure, styles, embedded images and new heading tags preserved.");
}
