using System.Buffers.Binary;
using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class EmfDimensionsTests
{
    internal static void Run()
    {
        void Check(bool value, string message) { if (!value) throw new Exception(message); }
        var root = new DirectoryInfo(Directory.GetCurrentDirectory());
        while (!File.Exists(Path.Combine(root.FullName, "examples", "emf", "EmfOnly.emf")))
            root = root.Parent ?? throw new Exception("Missing tracked EMF fixtures.");
        var resources = Path.Combine(root.FullName, "examples", "emf");
        var source = new TemplateSource(new XElement("P"), "slot", "", "");
        var profile = InvestigationTemplateProfile.FromTaggedTemplate(typeof(IrPreviewPlan).Assembly.Location, [], "reset", 142, 6, 0,
            new ProfileCaptionSelector("caption", "[", "]", "slot"), source, source,
            new TemplateListPrototype("bullet", 0, new XElement("BULLET")), new TemplateListPrototype("ordered", 0, new XElement("NUMBERING")));
        foreach (var name in new[] { "EmfOnly.emf", "EmfPlusDual.emf", "EmfPlusOnly.emf" })
        {
            Check(FigureDimensions.Read(Path.Combine(resources, name)) == (14175, 7975), "EMF physical frame lost.");
            using var input = JsonDocument.Parse($$"""{"type":"figure","image":{"path":"{{name}}","title":null,"alt":[]},"caption":[{"type":"text","value":"EMF"}],"source":null}""");
            var builder = new PlanBuilder(Path.Combine(resources, "fixture.ir.json"), resources, profile);
            builder.AddBlock(input.RootElement, "/blocks/0");
            var operation = builder.Build(1).Operations.Single();
            Check(operation.ImagePath == Path.Combine(resources, name) && operation.ImageWidthMillimeters == 142 &&
                Math.Abs(operation.ImageHeightMillimeters!.Value - 142.0 * 7975 / 14175) < 0.00001,
                "EMF resource path or physical aspect ratio lost.");
        }
        var temporary = Path.Combine(Path.GetTempPath(), "md2hwp-emf-" + Guid.NewGuid().ToString("N") + ".EMF");
        void Write(byte[] bytes) => File.WriteAllBytes(temporary, bytes);
        void U(byte[] bytes, int offset, uint value) => BinaryPrimitives.WriteUInt32LittleEndian(bytes.AsSpan(offset, 4), value);
        void I(byte[] bytes, int offset, int value) => BinaryPrimitives.WriteInt32LittleEndian(bytes.AsSpan(offset, 4), value);
        byte[] Minimal()
        {
            var bytes = new byte[108];
            U(bytes, 0, 1); U(bytes, 4, 88);
            I(bytes, 8, 0); I(bytes, 12, 0); I(bytes, 16, 99); I(bytes, 20, 49);
            I(bytes, 24, -2000); I(bytes, 28, 3000); I(bytes, 32, 12000); I(bytes, 36, 11000);
            U(bytes, 40, 0x464d4520); U(bytes, 44, 0x00010000); U(bytes, 48, 108); U(bytes, 52, 2);
            bytes[56] = 1;
            U(bytes, 72, 1920); U(bytes, 76, 1080); U(bytes, 80, 480); U(bytes, 84, 270);
            U(bytes, 88, 14); U(bytes, 92, 20); U(bytes, 104, 20);
            return bytes;
        }
        void Reject(byte[] bytes, string context)
        {
            Write(bytes);
            try { FigureDimensions.Read(temporary); }
            catch (InvalidDataException) { return; }
            throw new Exception("Invalid EMF accepted: " + context);
        }
        void Mutate(int offset, uint value, string context)
        {
            var bytes = Minimal(); U(bytes, offset, value); Reject(bytes, context);
        }
        try
        {
            Write(Minimal()); Check(FigureDimensions.Read(temporary) == (14000, 8000), "Uppercase extension or nonzero frame origin failed.");
            var extreme = Minimal(); I(extreme, 24, int.MinValue); I(extreme, 32, int.MaxValue);
            Write(extreme); Check(FigureDimensions.Read(temporary).Width == uint.MaxValue, "Frame subtraction overflowed.");
            Reject([], "empty"); Reject(Minimal()[..87], "truncated header"); Reject(Minimal()[..^1], "truncated EOF");
            Reject([..Minimal(), 0, 0, 0, 0], "trailing bytes");
            Mutate(0, 2, "signature/type"); Mutate(4, 84, "short header"); Mutate(4, 90, "unaligned header");
            Mutate(4, uint.MaxValue, "header boundary"); Mutate(40, 0, "EMF signature"); Mutate(44, 0, "version");
            Mutate(48, 100, "byte count"); Mutate(52, 0, "record count"); Mutate(52, 3, "missing records");
            Mutate(52, EmfDimensions.MaximumRecords + 1, "record limit");
            Mutate(32, unchecked((uint)-2000), "zero frame"); Mutate(36, 2000, "reversed frame");
            Mutate(60, uint.MaxValue, "description overflow");
            var description = Minimal(); U(description, 60, 1); U(description, 64, 87); Reject(description, "description boundary");
            Mutate(88, 0, "zero record type"); Mutate(88, 1, "duplicate header"); Mutate(88, 10, "missing EOF");
            Mutate(92, 4, "short record"); Mutate(92, 18, "unaligned record"); Mutate(92, 24, "record exceeds file");
            Mutate(104, 16, "EOF last size"); Mutate(96, uint.MaxValue, "palette overflow");
            var palette = Minimal(); U(palette, 96, 1); U(palette, 100, 16); Reject(palette, "palette boundary");
            using (var file = File.Create(temporary)) file.SetLength(EmfDimensions.MaximumFileBytes + 1);
            try { FigureDimensions.Read(temporary); throw new Exception("EMF size limit ignored."); }
            catch (InvalidDataException) { }
            Write(File.ReadAllBytes(Path.Combine(root.FullName, "examples", "jpeg", "image.jpg")));
            try { FigureDimensions.Read(temporary); throw new Exception("JPEG renamed EMF accepted."); }
            catch (InvalidDataException) { }
        }
        finally { File.Delete(temporary); }
        Console.WriteLine("EMF/EMF+ frame, resource plan, record/EOF boundaries, malformed input and bounded-container contracts passed.");
    }
}
