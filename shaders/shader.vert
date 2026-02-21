#version 450

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inTexCoord;

layout(location = 0) out vec3 color;
layout(location = 1) out vec2 fragTexCoord;

void main() {
    gl_Position = vec4(inPosition, 1.0) * ubo.model * ubo.view * ubo.proj;
    color = inColor;
    fragTexCoord = inTexCoord;
}
