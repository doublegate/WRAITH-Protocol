// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "WraithiOS",
    platforms: [
        .iOS(.v16)
    ],
    products: [
        .library(
            name: "WraithiOS",
            targets: ["WraithiOS"]
        ),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "WraithiOS",
            dependencies: ["WraithFFI"],
            path: "WraithiOS/Sources"
        ),
        .binaryTarget(
            name: "WraithFFI",
            path: "./wraith-swift-ffi/target/WraithFFI.xcframework"
        ),
        .testTarget(
            name: "WraithiOSTests",
            dependencies: ["WraithiOS"],
            path: "WraithiOS/Tests"
        ),
    ]
)
