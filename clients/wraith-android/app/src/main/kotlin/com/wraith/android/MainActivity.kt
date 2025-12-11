package com.wraith.android

import android.Manifest
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.lifecycleScope
import com.wraith.android.ui.theme.WraithAndroidTheme
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    private val wraith = WraithClient()

    private val permissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        val allGranted = permissions.values.all { it }
        if (allGranted) {
            startWraith()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Request permissions
        permissionLauncher.launch(arrayOf(
            Manifest.permission.INTERNET,
            Manifest.permission.ACCESS_NETWORK_STATE,
            Manifest.permission.READ_EXTERNAL_STORAGE,
            Manifest.permission.WRITE_EXTERNAL_STORAGE,
            Manifest.permission.FOREGROUND_SERVICE,
        ))

        setContent {
            WraithAndroidTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    MainScreen(wraith = wraith)
                }
            }
        }
    }

    private fun startWraith() {
        lifecycleScope.launch {
            try {
                wraith.start()
            } catch (e: WraithException) {
                // Handle error
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        lifecycleScope.launch {
            wraith.shutdown()
        }
    }
}

@Composable
fun MainScreen(wraith: WraithClient) {
    var nodeStatus by remember { mutableStateOf<NodeStatus?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        try {
            nodeStatus = wraith.getStatus()
        } catch (e: WraithException) {
            error = e.message
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(
            text = "WRAITH Protocol",
            style = MaterialTheme.typography.headlineMedium
        )

        Spacer(modifier = Modifier.height(32.dp))

        nodeStatus?.let { status ->
            StatusCard(status = status)
        }

        error?.let { err ->
            Text(
                text = "Error: $err",
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodyMedium
            )
        }
    }
}

@Composable
fun StatusCard(status: NodeStatus) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 4.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text(
                text = "Node Status",
                style = MaterialTheme.typography.titleMedium
            )

            Divider()

            StatusRow(label = "Status", value = if (status.running) "Running" else "Stopped")
            StatusRow(label = "Peer ID", value = status.localPeerId.take(16) + "...")
            StatusRow(label = "Sessions", value = status.sessionCount.toString())
            StatusRow(label = "Transfers", value = status.activeTransfers.toString())
        }
    }
}

@Composable
fun StatusRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(text = label, style = MaterialTheme.typography.bodyMedium)
        Text(text = value, style = MaterialTheme.typography.bodyMedium)
    }
}
