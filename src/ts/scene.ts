import { gl } from "./gl-utils/gl-canvas";
import { Shader } from "./gl-utils/shader";

import { asyncLoadObjModel, ObjModel } from "./obj-model";
import { Parameters } from "./parameters";
import { RenderToTextureWithDepth } from "./texture/render-to-texture-with-depth";
import { asyncLoadShader } from "./utils";


interface IScenePreset {
    modelName: string;
    cameraPosition: [number, number, number];
    nearPlane: number;
    farPlane: number;
    model?: ObjModel; // "undefined" if not loaded, null if loading, ObjModel if loaded
}

const CUSTOM_VIEW = {
    cameraPosition: [-2, 0, -1] as [number, number, number],
    nearPlane: 1.3,
    farPlane: 3,
};

const presets: { [id: string]: IScenePreset } = {
    "primitives": {
        modelName: "primitives.obj",
        cameraPosition: [-1.3, 0, -0.54],
        nearPlane: 0.6,
        farPlane: 2.2,
    },
    "cube": {
        modelName: "cube.obj",
        cameraPosition: [-2, 0, -1],
        nearPlane: 1.3,
        farPlane: 3,
    },
    "monkey": {
        modelName: "monkey.obj",
        cameraPosition: [-1.3, 0, -0.5],
        nearPlane: 0.7,
        farPlane: 2,
    },
    "bunny": {
        modelName: "bunny.obj",
        cameraPosition: [-1.7, 0, -0.8],
        nearPlane: 0.9,
        farPlane: 2.3,
    },
    "atomium": {
        modelName: "atomium.obj",
        cameraPosition: [-2.5, 0, -0.6],
        nearPlane: 1.5,
        farPlane: 3.4,
    }
};

declare const mat4: any;

class Scene {
    private readonly _depthMap: RenderToTextureWithDepth;
    private customModel: ObjModel | null = null;

    private readonly modelMatrix: any;
    private readonly viewMatrix: any;
    private readonly projectionMatrix: any;
    private readonly mvMatrix: any;
    private readonly mvpMatrix: any;

    private shader: Shader;

    public constructor() {
        this._depthMap = new RenderToTextureWithDepth();

        this.modelMatrix = mat4.create();
        this.viewMatrix = mat4.create();
        this.projectionMatrix = mat4.create();
        this.mvMatrix = mat4.create();
        this.mvpMatrix = mat4.create();

        asyncLoadShader("scene", "scene.vert", "scene.frag", (shader: Shader) => {
            shader.u["uMVPMatrix"].value = this.mvpMatrix;
            this.shader = shader;
        });

        Parameters.customMeshObservers.push((positions: Float32Array | null) => {
            this.setCustomMesh(positions);
        });
    }

    public computeDepthMap(width: number, height: number): void {
        if (!this.shader) {
            return;
        }

        const view = this.resolveView();
        if (!view) {
            return;
        }

        mat4.lookAt(this.viewMatrix, view.cameraPosition, [0, 0, 0], [0, 0, 1]);

        mat4.fromRotation(this.modelMatrix, 0.0005 * performance.now(), [0, 0, 1]);
        mat4.multiply(this.mvMatrix, this.viewMatrix, this.modelMatrix);

        mat4.perspective(this.projectionMatrix, 45, width / height, view.nearPlane, view.farPlane);
        mat4.multiply(this.mvpMatrix, this.projectionMatrix, this.mvMatrix);

        this._depthMap.reserveSpace(width, height);
        gl.bindFramebuffer(gl.FRAMEBUFFER, this._depthMap.framebuffer);
        gl.viewport(0, 0, this._depthMap.width, this._depthMap.height);
        gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

        const model = this.resolveModel();
        if (model) {
            this.shader.a["aVertice"].VBO = model.VBO;
            this.shader.use();
            this.shader.bindUniformsAndAttributes();

            gl.drawArrays(gl.TRIANGLES, 0, 3 * model.trianglesCount);
        }
    }

    private setCustomMesh(positions: Float32Array | null): void {
        if (this.customModel) {
            this.customModel.VBO.freeGLResources();
            this.customModel = null;
        }

        if (positions) {
            this.customModel = new ObjModel(positions);
        }
    }

    private resolveView(): { cameraPosition: [number, number, number]; nearPlane: number; farPlane: number } | null {
        if (!Parameters.modelId) {
            return CUSTOM_VIEW;
        }

        return presets[Parameters.modelId] ?? null;
    }

    private resolveModel(): ObjModel | null {
        if (!Parameters.modelId) {
            return this.customModel;
        }

        const modelPreset = presets[Parameters.modelId];
        if (!modelPreset) {
            return null;
        }

        if (typeof modelPreset.model === "undefined") {
            modelPreset.model = null;
            const loadedModelId = Parameters.modelId;
            asyncLoadObjModel(modelPreset.modelName, (loaded: ObjModel) => {
                if (loadedModelId === Parameters.modelId) { // this method is called asynchronously, so check that this is still the model we want
                    modelPreset.model = loaded;
                }
            });
        }

        return modelPreset.model ?? null;
    }

    public get depthMap(): RenderToTextureWithDepth {
        return this._depthMap;
    }
}

export {
    Scene,
};

