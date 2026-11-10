// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
using System.Reflection;
using Md2Hwp.HancomIrPreview;

namespace Md2Hwp.EquationRecalc;

internal static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        try
        {
            if (args is ["--help"] or ["-h"])
            {
                Console.WriteLine(EquationRecalculationOptions.Usage);
                return 0;
            }
            if (args is ["--version"])
            {
                var version = typeof(Program).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()
                    ?.InformationalVersion ?? throw new InvalidOperationException("Missing application version metadata.");
                Console.WriteLine($"hwpeqrecalc {version.Split('+')[0]}");
                return 0;
            }
            var result = HancomPreviewWriter.RecalculateEquations(EquationRecalculationOptions.Parse(args));
            foreach (var warning in result.Warnings) Console.Error.WriteLine($"hwpeqrecalc: warning: {warning}");
            Console.Error.WriteLine($"hwpeqrecalc: {result.Equations} equation(s), {result.Resized} resized; " +
                $"{result.TablesResized} table height(s) refreshed by the re-layout; " +
                $"{result.ImagesReencoded} embedded image(s) re-encoded by Hancom on save");
            Console.WriteLine(result.Output);
            return 0;
        }
        catch (Exception error)
        {
            // The Hancom session wraps failures; report the outermost specific one.
            var reported = error;
            while (reported.InnerException is { } inner && reported.Message == "Hancom preview operation failed.") reported = inner;
            Console.Error.WriteLine($"hwpeqrecalc: {reported.Message}");
            return 1;
        }
    }
}
