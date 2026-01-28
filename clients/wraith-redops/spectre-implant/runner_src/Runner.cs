using System;
using System.IO;
using System.Text;
using System.Management.Automation;
using System.Management.Automation.Runspaces;
using System.Collections.ObjectModel;

namespace Wraith
{
    public class Runner
    {
        public static int Run(string args)
        {
            try
            {
                string[] parts = args.Split('\n');
                if (parts.Length < 2) return -1;

                string outputPath = parts[0];
                string b64Cmd = parts[1];
                string script = Encoding.UTF8.GetString(Convert.FromBase64String(b64Cmd));

                string output = "";

                using (Runspace runspace = RunspaceFactory.CreateRunspace())
                {
                    runspace.Open();
                    using (PowerShell ps = PowerShell.Create())
                    {
                        ps.Runspace = runspace;
                        ps.AddScript(script);
                        
                        Collection<PSObject> results = ps.Invoke();
                        
                        StringBuilder sb = new StringBuilder();
                        foreach (PSObject obj in results)
                        {
                            sb.AppendLine(obj.ToString());
                        }
                        
                        if (ps.Streams.Error.Count > 0)
                        {
                            sb.AppendLine("ERRORS:");
                            foreach (ErrorRecord err in ps.Streams.Error)
                            {
                                sb.AppendLine(err.ToString());
                            }
                        }
                        
                        output = sb.ToString();
                    }
                    runspace.Close();
                }

                File.WriteAllText(outputPath, output);
                return 0;
            }
            catch (Exception ex)
            {
                // In case of catastrophic failure, try to write the exception
                try {
                    string[] parts = args.Split('\n');
                    if (parts.Length >= 1) {
                        File.WriteAllText(parts[0], "Runner Exception: " + ex.ToString());
                    }
                } catch {}
                return -99;
            }
        }
    }
}
