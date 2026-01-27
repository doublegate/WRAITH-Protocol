using System;
using System.IO;
using System.Management.Automation;
using System.Collections.ObjectModel;
using System.Text;

namespace Wraith
{
    public class Runner
    {
        public static int Run(string input)
        {
            string outputPath = "unknown";
            try
            {
                // Format: OutputPath|Base64Command
                int splitIndex = input.IndexOf('|');
                if (splitIndex == -1) return -2;

                outputPath = input.Substring(0, splitIndex);
                string base64Cmd = input.Substring(splitIndex + 1);
                
                string cmd = Encoding.UTF8.GetString(Convert.FromBase64String(base64Cmd));

                StringBuilder outputBuilder = new StringBuilder();

                using (PowerShell ps = PowerShell.Create())
                {
                    ps.AddScript(cmd);
                    Collection<PSObject> results = ps.Invoke();

                    if (ps.Streams.Error.Count > 0)
                    {
                        foreach (var err in ps.Streams.Error)
                        {
                            outputBuilder.AppendLine("ERROR: " + err.ToString());
                        }
                    }

                    foreach (PSObject obj in results)
                    {
                        if (obj != null)
                            outputBuilder.AppendLine(obj.ToString());
                    }
                }

                File.WriteAllText(outputPath, outputBuilder.ToString());
                return 0;
            }
            catch (Exception e)
            {
                try { 
                    if (outputPath != "unknown")
                        File.WriteAllText(outputPath, "EXCEPTION: " + e.ToString()); 
                } catch { }
                return -1;
            }
        }
    }
}
