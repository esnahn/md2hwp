using System.Text.Json;

namespace Md2Hwp.HancomIrPreview;

internal static class IrContract
{
    // Shared with Rust at build time; no runtime schema sidecar is required.
    public static readonly string Version = ReadVersion();

    public const string UnsupportedListMath = "Hancom generation does not yet support display math in list items";

    // Same minimal check as the Rust core: SCRIPT grammar is not parsed.
    public static string ReadMathScript(JsonElement element, string path)
    {
        JsonContract.ExpectObject(element, path, ["type", "script"]);
        var script = JsonContract.RequiredString(element, "script", path);
        if (string.IsNullOrWhiteSpace(script))
            throw JsonContract.Error(path + "/script", "equation script must not be empty");
        if (!script.IsNormalized(System.Text.NormalizationForm.FormC))
            throw JsonContract.Error(path + "/script", "equation script must use Unicode NFC");
        if (script.Any(c => c < ' ' || c == '\u007f'))
            throw JsonContract.Error(path + "/script", "equation script contains a control character");
        return script;
    }

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
