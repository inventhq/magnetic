// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "MagneticSDK",
    platforms: [
        .iOS(.v16),
        .macOS(.v13),
    ],
    products: [
        .library(
            name: "MagneticSDK",
            targets: ["MagneticSDK"]
        ),
    ],
    targets: [
        .target(
            name: "MagneticSDK",
            path: "Sources/MagneticSDK"
        ),
        .testTarget(
            name: "MagneticSDKTests",
            dependencies: ["MagneticSDK"],
            path: "Tests/MagneticSDKTests"
        ),
    ]
)
