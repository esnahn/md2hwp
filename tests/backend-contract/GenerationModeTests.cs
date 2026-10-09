using Md2Hwp.HancomIrPreview;

internal static class GenerationModeTests
{
    public static void Run()
    {
        var positional = new[] { "source.ir.json", "output.hwp", "--template", "custom.hwp" };
        var explicitArgs = new[] { "ir2hwp", "--ir", "source.ir.json", "--output", "output.hwp", "--template", "custom.hwp" };
        foreach (var args in new[] { positional, explicitArgs })
        {
#if DEBUG
            if (CommandLine.Parse(args).LegacyCom) throw new Exception("XML composition must be the default.");
            var legacy = CommandLine.Parse([.. args, "--legacy-com", "--verbose"]);
            if (!legacy.LegacyCom || !legacy.Verbose) throw new Exception("Legacy flag was not forwarded.");
            Reject(() => CommandLine.Parse([.. args, "--legacy-com", "--legacy-com"]), "Duplicate");
#else
            _ = CommandLine.Parse(args);
            Reject(() => CommandLine.Parse([.. args, "--legacy-com"]), "usage:");
#endif
        }
#if !DEBUG
        if (CommandLine.Usage.Contains("legacy", StringComparison.OrdinalIgnoreCase))
            throw new Exception("Release help must not advertise legacy generation.");
        if (typeof(CommandLine).GetProperty("LegacyCom") is not null ||
            typeof(HancomPreviewWriter).GetMethod("RenderTaggedTemplate")!.GetParameters().Any(p => p.Name == "legacyCom"))
            throw new Exception("Release must not expose a legacy rendering mode.");
#else
        if (!CommandLine.Usage.Contains("--legacy-com", StringComparison.Ordinal))
            throw new Exception("Debug help must advertise its comparison baseline.");
#endif
        Console.WriteLine("Generation mode defaults, flag forwarding and configuration restrictions passed.");
    }

    private static void Reject(Action action, string message)
    {
        try { action(); }
        catch (ArgumentException error) when (error.Message.Contains(message, StringComparison.Ordinal)) { return; }
        throw new Exception("Expected generation mode argument rejection: " + message);
    }
}
