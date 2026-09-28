using System.Text.Json;

namespace Md2Hwp.HancomIrPreview;

internal static class IrContract
{
    // Shared with Rust at build time; no runtime schema sidecar is required.
    public static readonly string Version = ReadVersion();

    private static string ReadVersion()
    {
        using var stream = typeof(IrContract).Assembly.GetManifestResourceStream("md2hwp.ir.schema")
            ?? throw new InvalidOperationException("Missing embedded IR schema.");
        using var schema = JsonDocument.Parse(stream);
        return schema.RootElement.GetProperty("properties").GetProperty("ir_version").GetProperty("const").GetString()
            ?? throw new InvalidOperationException("Missing schema IR version.");
    }

    public static void RequireCurrent(string version, string kind)
    {
        if (version != Version)
            throw new InvalidDataException($"{kind} IR version {version} does not match this program ({Version}). " +
                "Regenerate IR from the manuscript and create a new template with init-template.");
    }
}
