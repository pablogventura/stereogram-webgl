const STL_HEADER_BYTES = 80;
const STL_COUNT_BYTES = 4;
const STL_TRIANGLE_BYTES = 50;
const MAX_TRIANGLES = 2_000_000;
const NORMALIZED_SIZE = 1;

class StlParseError extends Error {
    public constructor(message: string) {
        super(message);
        this.name = "StlParseError";
    }
}

// Binary STL matches its declared size. Anything else is treated as ASCII,
// including files whose 80-byte header happens to start with "solid".
function isBinaryStl(buffer: ArrayBuffer): boolean {
    if (buffer.byteLength < STL_HEADER_BYTES + STL_COUNT_BYTES) {
        return false;
    }

    const triangleCount = new DataView(buffer).getUint32(STL_HEADER_BYTES, true);
    const expectedLength = STL_HEADER_BYTES + STL_COUNT_BYTES + triangleCount * STL_TRIANGLE_BYTES;
    return triangleCount > 0 && buffer.byteLength === expectedLength;
}

function assertTriangleCount(triangleCount: number): void {
    if (triangleCount > MAX_TRIANGLES) {
        throw new StlParseError("El archivo STL tiene demasiados triángulos.");
    }
}

function parseBinaryStl(buffer: ArrayBuffer): Float32Array {
    const view = new DataView(buffer);
    const triangleCount = view.getUint32(STL_HEADER_BYTES, true);
    assertTriangleCount(triangleCount);

    const positions = new Float32Array(triangleCount * 9);
    let offset = STL_HEADER_BYTES + STL_COUNT_BYTES;
    let writeIndex = 0;

    for (let triangleIndex = 0; triangleIndex < triangleCount; triangleIndex++) {
        offset += 12; // normal
        for (let vertexIndex = 0; vertexIndex < 3; vertexIndex++) {
            positions[writeIndex++] = view.getFloat32(offset, true);
            positions[writeIndex++] = view.getFloat32(offset + 4, true);
            positions[writeIndex++] = view.getFloat32(offset + 8, true);
            offset += 12;
        }
        offset += 2; // attribute byte count
    }

    return positions;
}

function parseAsciiStl(buffer: ArrayBuffer): Float32Array {
    const text = new TextDecoder("utf-8").decode(buffer);
    const values: number[] = [];
    const lines = text.split(/\r?\n/);

    for (const rawLine of lines) {
        const line = rawLine.trim();
        if (!line.toLowerCase().startsWith("vertex")) {
            continue;
        }

        const parts = line.split(/\s+/);
        if (parts.length < 4) {
            throw new StlParseError("El archivo STL tiene un vértice inválido.");
        }

        const x = Number(parts[1]);
        const y = Number(parts[2]);
        const z = Number(parts[3]);
        if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) {
            throw new StlParseError("El archivo STL tiene un vértice inválido.");
        }

        values.push(x, y, z);
        assertTriangleCount(values.length / 9);
    }

    return new Float32Array(values);
}

function normalizePositions(positions: Float32Array): Float32Array {
    let minX = Infinity;
    let minY = Infinity;
    let minZ = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    let maxZ = -Infinity;

    for (let index = 0; index < positions.length; index += 3) {
        const x = positions[index];
        const y = positions[index + 1];
        const z = positions[index + 2];
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        minZ = Math.min(minZ, z);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
        maxZ = Math.max(maxZ, z);
    }

    const centerX = (minX + maxX) / 2;
    const centerY = (minY + maxY) / 2;
    const centerZ = (minZ + maxZ) / 2;
    const longestSide = Math.max(maxX - minX, maxY - minY, maxZ - minZ);
    if (!Number.isFinite(longestSide)) {
        throw new StlParseError("El archivo STL tiene un vértice inválido.");
    }

    const scale = longestSide > 0 ? NORMALIZED_SIZE / longestSide : 1;
    for (let index = 0; index < positions.length; index += 3) {
        positions[index] = (positions[index] - centerX) * scale;
        positions[index + 1] = (positions[index + 1] - centerY) * scale;
        positions[index + 2] = (positions[index + 2] - centerZ) * scale;
    }

    return positions;
}

function parseStl(buffer: ArrayBuffer): Float32Array {
    const positions = isBinaryStl(buffer) ? parseBinaryStl(buffer) : parseAsciiStl(buffer);
    if (positions.length === 0 || positions.length % 9 !== 0) {
        throw new StlParseError("El archivo STL no tiene triángulos.");
    }

    return normalizePositions(positions);
}

export {
    parseStl,
    StlParseError,
};
