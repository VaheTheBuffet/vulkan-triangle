use glfw::ffi::*;
use vulkan::vulkan as vk;
use std::ptr::{null, null_mut};
use std::collections::{BTreeMap, HashSet};
use obj;
use crate::math;

const WIDTH:i32 = 800;
const HEIGHT:i32 = 600;
const MAX_FRAMES_IN_FLIGHT:usize = 2;

const VALIDATION_LAYERS: [&str; 1] = [
    "VK_LAYER_KHRONOS_validation"
];

const DEVICE_EXTENSIONS: [*const u8; 1] = [
    vk::VK_KHR_SWAPCHAIN_EXTENSION_NAME.as_ptr()
];

#[cfg(debug_assertions)]
const ENABLE_VALIDATION_LAYERS: bool = true;

#[cfg(not(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = false;

struct UniformBufferObject {
    model: [f32; 16],
    view: [f32; 16],
    proj: [f32; 16]
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    texcoord: [f32; 2]
}

impl Vertex {
    fn get_binding_description() -> vk::VkVertexInputBindingDescription {
        let mut binding_description = vk::VkVertexInputBindingDescription::default();
        binding_description.binding = 0;
        binding_description.stride = size_of::<Vertex>() as _;
        binding_description.inputRate = vk::VK_VERTEX_INPUT_RATE_VERTEX;

        binding_description
    }

    fn get_attribute_descriptions() -> Vec<vk::VkVertexInputAttributeDescription> {
        let mut attribute_descriptions = vec![vk::VkVertexInputAttributeDescription::default(); 3];

        attribute_descriptions[0].binding = 0;
        attribute_descriptions[0].location = 0;
        attribute_descriptions[0].format = vk::VK_FORMAT_R32G32B32_SFLOAT;
        attribute_descriptions[0].offset = 0;

        attribute_descriptions[1].binding = 0;
        attribute_descriptions[1].location = 1;
        attribute_descriptions[1].format = vk::VK_FORMAT_R32G32B32_SFLOAT;
        attribute_descriptions[1].offset = std::mem::offset_of!(Vertex, color) as _;
        
        attribute_descriptions[2].binding = 0;
        attribute_descriptions[2].location = 2;
        attribute_descriptions[2].format = vk::VK_FORMAT_R32G32_SFLOAT;
        attribute_descriptions[2].offset = std::mem::offset_of!(Vertex, texcoord) as _;
        attribute_descriptions
    }
}

impl std::hash::Hash for Vertex {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe {
            (*std::mem::transmute::<_, &(u32, u32, u32)>(&self.position)).hash(state);
            (*std::mem::transmute::<_, &(u32, u32, u32)>(&self.color)).hash(state);
            (*std::mem::transmute::<_, &(u32, u32)>(&self.texcoord)).hash(state);
        }
    }
}

impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        unsafe{
            *std::mem::transmute::<_, &[u8; size_of::<Vertex>()]>(self) 
            == *std::mem::transmute::<_, &[u8; size_of::<Vertex>()]>(other) 
        } 
    }
}

impl Eq for Vertex {}


#[derive(Default)]
pub struct HelloTriangleApplication {
    window: *mut GLFWwindow,
    instance: vk::VkInstance,
    debug_messenger: vk::VkDebugUtilsMessengerEXT,
    physical_device: vk::VkPhysicalDevice,
    device: vk::VkDevice,
    graphics_queue: vk::VkQueue,
    transfer_queue: vk::VkQueue,
    surface: vk::VkSurfaceKHR,
    present_queue: vk::VkQueue,
    swap_chain: vk::VkSwapchainKHR,
    swap_chain_images: Vec<vk::VkImage>,
    swap_chain_image_format: vk::VkFormat,
    swap_chain_extent: vk::VkExtent2D,
    swap_chain_image_views: Vec<vk::VkImageView>,
    render_pass: vk::VkRenderPass,
    descriptor_set_layout: vk::VkDescriptorSetLayout,
    descriptor_sets: Vec<vk::VkDescriptorSet>,
    pipeline_layout: vk::VkPipelineLayout,
    pipeline: vk::VkPipeline,
    swap_chain_framebuffers: Vec<vk::VkFramebuffer>,
    command_pool: vk::VkCommandPool,
    transfer_command_pool: vk::VkCommandPool,
    command_buffers: Vec<vk::VkCommandBuffer>,
    image_available_semaphores: Vec<vk::VkSemaphore>,
    render_finished_semaphores: Vec<vk::VkSemaphore>,
    in_flight_fences: Vec<vk::VkFence>,
    current_frame: usize,
    framebuffer_resized: bool,

    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    vertex_buffer: vk::VkBuffer,
    vertex_buffer_memory: vk::VkDeviceMemory,
    index_buffer: vk::VkBuffer,
    index_buffer_memory: vk::VkDeviceMemory,

    descriptor_pool: vk::VkDescriptorPool,

    uniform_buffers: Vec<vk::VkBuffer>,
    uniform_buffers_memory: Vec<vk::VkDeviceMemory>,
    uniform_buffers_memory_mapped: Vec<*mut std::ffi::c_void>,

    texture_image: vk::VkImage,
    mip_levels: u32,
    texture_image_memory: vk::VkDeviceMemory,
    texture_image_views: vk::VkImageView,
    texture_sampler: vk::VkSampler,

    depth_image: vk::VkImage,
    depth_image_memory: vk::VkDeviceMemory,
    depth_view: vk::VkImageView,

    msaa_samples: vk::VkSampleCountFlagBits,

    color_image: vk::VkImage,
    color_image_memory: vk::VkDeviceMemory,
    color_image_view: vk::VkImageView
}

impl HelloTriangleApplication {
    pub fn run(&mut self) {
        self.init_window();
        self.init_vulkan();
        self.main_loop();
        self.cleanup();
    }


    fn init_window(&mut self) {
        unsafe {
            glfwInit();
            glfwWindowHint(GLFW_CLIENT_API, GLFW_NO_API);

            self.window = glfwCreateWindow(
                WIDTH, 
                HEIGHT, 
                "VK_app\0".as_ptr() as _, 
                std::ptr::null_mut(), 
                std::ptr::null_mut()
            );

            glfwSetWindowUserPointer(self.window, self as *mut _ as _);
            //glfwSetFramebufferSizeCallback(self.window, Some(framebufferResizeCallback));
        }
    }


    fn init_vulkan(&mut self) {
        self.create_instance();
        self.setup_debug_messanger();
        self.create_surface();
        self.pick_physical_device();
        self.create_logical_device();
        self.create_swapchain();
        self.create_command_pools();
        self.create_image_views();
        self.create_descriptor_set_layout();

        self.create_color_resources();
        self.create_depth_resources();

        self.create_render_pass();
        self.create_graphics_pipeline();
        self.create_framebuffers();

        self.load_models();
        self.create_vertex_buffer();
        self.create_index_buffer();

        self.create_uniform_buffers();
        self.create_descriptor_pool();
        self.create_command_buffers();
        self.create_sync_objects();
        self.create_texture_image();
        self.create_texture_image_view();
        self.create_texture_sampler();
        self.create_descriptor_sets();
    }


    fn  main_loop(&mut self) {
        let mut n_frames = 0.0;
        let mut time = std::time::Instant::now();
        let mut one_second = 1f32;
        unsafe {
            while glfwWindowShouldClose(self.window) == 0 {
                one_second -= time.elapsed().as_secs_f32();
                time = std::time::Instant::now();
                n_frames += 1.0;
                if one_second < 0.0 {
                    one_second = 1.0;
                    glfwSetWindowTitle(self.window, format!("VK_app {}\0", n_frames).as_ptr() as _);
                    n_frames = 0.0;
                }
                glfwPollEvents();
                self.draw_frame();
            }

            vk::vkDeviceWaitIdle(self.device);
        }
    }


    fn cleanup(&mut self) {
        unsafe {
            self.cleanup_swapchain();
            vk::vkDestroySampler(self.device, self.texture_sampler, null());
            vk::vkDestroyImageView(self.device, self.texture_image_views, null());
            vk::vkDestroyImage(self.device, self.texture_image, null());
            vk::vkFreeMemory(self.device, self.texture_image_memory, null());
            vk::vkDestroyBuffer(self.device, self.vertex_buffer, null());
            vk::vkDestroyBuffer(self.device, self.index_buffer, null());
            vk::vkFreeMemory(self.device, self.vertex_buffer_memory, null());
            vk::vkFreeMemory(self.device, self.index_buffer_memory, null());
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                vk::vkDestroyBuffer(self.device, self.uniform_buffers[i], null());
                vk::vkFreeMemory(self.device, self.uniform_buffers_memory[i], null());
            }
            vk::vkDestroyDescriptorPool(self.device, self.descriptor_pool, null());
            vk::vkDestroyDescriptorSetLayout(self.device, self.descriptor_set_layout, null());
            for semaphore in self.render_finished_semaphores.iter() {
                vk::vkDestroySemaphore(self.device, *semaphore, null());
            }
            for semaphore in self.image_available_semaphores.iter() {
                vk::vkDestroySemaphore(self.device, *semaphore, null());
            }
            for fence in self.in_flight_fences.iter() {
                vk::vkDestroyFence(self.device, *fence, null());
            }
            vk::vkDestroyCommandPool(self.device, self.command_pool, null());
            vk::vkDestroyCommandPool(self.device, self.transfer_command_pool, null());
            vk::vkDestroyPipeline(self.device, self.pipeline, null());
            vk::vkDestroyPipelineLayout(self.device, self.pipeline_layout, null());
            vk::vkDestroyRenderPass(self.device, self.render_pass, null());
            vk::vkDestroyDevice(self.device, null());
            if ENABLE_VALIDATION_LAYERS {
                destroy_debug_utils_messenger(self.instance, self.debug_messenger, null());
            }
            vk::vkDestroySurfaceKHR(self.instance, self.surface, null());
            vk::vkDestroyInstance(self.instance, null());
            glfwDestroyWindow(self.window);
            glfwTerminate();
        }
    }


    fn create_color_resources(&mut self) {
        let color_format = self.swap_chain_image_format;

        let this = unsafe{&mut *(self as *mut Self)};
        self.create_image(
            self.swap_chain_extent.width, 
            self.swap_chain_extent.height, 
            1, 
            self.msaa_samples, 
            color_format, 
            vk::VK_IMAGE_TILING_OPTIMAL, 
            vk::VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT | vk::VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT, 
            vk::VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT as u32, 
            &mut this.color_image, 
            &mut this.color_image_memory
        );

        self.color_image_view = self.create_image_view(
            self.color_image, 
            color_format, 
            vk::VK_IMAGE_ASPECT_COLOR_BIT as _, 1
        );
    }


    fn get_max_usable_sample_count(&self) -> vk::VkSampleCountFlagBits {
        let mut device_properties = vk::VkPhysicalDeviceProperties::default();
        unsafe {
            vk::vkGetPhysicalDeviceProperties(self.physical_device, &mut device_properties);
        }
        let counts =device_properties.limits.framebufferColorSampleCounts 
                    & device_properties.limits.framebufferDepthSampleCounts;

        if counts & vk::VK_SAMPLE_COUNT_64_BIT as u32 != 0 {
            vk::VK_SAMPLE_COUNT_64_BIT
        } else if counts & vk::VK_SAMPLE_COUNT_32_BIT as u32 != 0 {
            vk::VK_SAMPLE_COUNT_32_BIT
        } else if counts & vk::VK_SAMPLE_COUNT_16_BIT as u32 != 0 {
            vk::VK_SAMPLE_COUNT_16_BIT
        } else if counts & vk::VK_SAMPLE_COUNT_8_BIT as u32 != 0 {
            vk::VK_SAMPLE_COUNT_8_BIT
        } else if counts & vk::VK_SAMPLE_COUNT_4_BIT as u32 != 0 {
            vk::VK_SAMPLE_COUNT_4_BIT
        } else if counts & vk::VK_SAMPLE_COUNT_2_BIT as u32 != 0 {
            vk::VK_SAMPLE_COUNT_2_BIT
        } else {
            vk::VK_SAMPLE_COUNT_1_BIT
        }
    }


    fn load_models(&mut self) {
        self.load_model("assets/models/viking_room.obj", false);

    }


    fn load_model(&mut self, name: &str, counter_clockwise_winding_order: bool) {
        let mut buf = obj::Obj::load(name).expect("failed to read file");
        let mut unique_vertices = std::collections::HashMap::<Vertex, u32>::new();
        self.vertices.reserve(buf.data.position.len());
        self.indices.reserve(buf.data.position.len());

        for poly in &mut buf.data.objects[0].groups[0].polys {
            let poly_iter: &mut dyn Iterator<Item = &obj::IndexTuple> = if counter_clockwise_winding_order {
                &mut poly.0.iter()
            } else {
                &mut poly.0.iter().rev()
            };

            for v_obj in poly_iter {
                let mut v_mesh = Vertex::default();
                v_mesh.position = buf.data.position[v_obj.0];
                v_mesh.texcoord = buf.data.texture[v_obj.1.expect("could not find texture")];
                v_mesh.texcoord[1] = 1.0 - v_mesh.texcoord[1];
                v_mesh.color = [0.0, 0.0, 0.0];

                if !unique_vertices.contains_key(&v_mesh) {
                    unique_vertices.insert(v_mesh, unique_vertices.len() as u32);
                    self.vertices.push(v_mesh)
                }

                self.indices.push(*unique_vertices.get(&v_mesh).unwrap())
            }
        }
        self.vertices.shrink_to_fit();
    }


    fn create_depth_resources(&mut self) {
        let format = self.find_depth_format();
        let this = unsafe{&mut *(self as *mut Self)};
        self.create_image(
            self.swap_chain_extent.width, 
            self.swap_chain_extent.height, 
            1,
            self.msaa_samples,
            format, 
            vk::VK_IMAGE_TILING_OPTIMAL, 
            vk::VK_IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT, 
            vk::VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT as _, 
            &mut this.depth_image, 
            &mut this.depth_image_memory
        );

        self.depth_view = self.create_image_view(self.depth_image, format, vk::VK_IMAGE_ASPECT_DEPTH_BIT as _, 1);

        //self.transition_image_layout(
            //self.depth_image, 
            //format, 
            //vk::VK_IMAGE_LAYOUT_UNDEFINED, 
            //vk::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            //1,
        //);
    }


    fn find_supported_format(
        &self, 
        candidates: &Vec<vk::VkFormat>, 
        tiling: vk::VkImageTiling, 
        features: vk::VkFormatFeatureFlags
    ) -> vk::VkFormat {

        for format in candidates {
            let mut props = vk::VkFormatProperties::default();
            unsafe{vk::vkGetPhysicalDeviceFormatProperties(self.physical_device, *format, &mut props)};

            if tiling == vk::VK_IMAGE_TILING_LINEAR && (props.linearTilingFeatures & features == features) {
                return *format;
            } else if tiling == vk::VK_IMAGE_TILING_OPTIMAL && (props.optimalTilingFeatures & features == features)  {
                return *format;
            }
        }

        panic!("failed to find supported depth buffer format");
    }


    fn find_depth_format(&self) -> vk::VkFormat {
        self.find_supported_format(
            &vec![vk::VK_FORMAT_D32_SFLOAT, vk::VK_FORMAT_D32_SFLOAT_S8_UINT, vk::VK_FORMAT_D24_UNORM_S8_UINT], 
            vk::VK_IMAGE_TILING_OPTIMAL, 
            vk::VK_FORMAT_FEATURE_DEPTH_STENCIL_ATTACHMENT_BIT as _
        )
    }


    fn create_texture_sampler(&mut self) {
        let mut sampler_info = vk::VkSamplerCreateInfo::default();
        sampler_info.sType = vk::VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO;
        sampler_info.magFilter = vk::VK_FILTER_LINEAR;
        sampler_info.minFilter = vk::VK_FILTER_LINEAR;
        sampler_info.addressModeU = vk::VK_SAMPLER_ADDRESS_MODE_REPEAT;
        sampler_info.addressModeV = vk::VK_SAMPLER_ADDRESS_MODE_REPEAT;
        sampler_info.addressModeW = vk::VK_SAMPLER_ADDRESS_MODE_REPEAT;
        sampler_info.anisotropyEnable = vk::VK_FALSE;
        sampler_info.maxAnisotropy = 1.0;
        sampler_info.borderColor = vk::VK_BORDER_COLOR_INT_OPAQUE_BLACK;

        let mut properties = vk::VkPhysicalDeviceProperties::default();
        unsafe {
            vk::vkGetPhysicalDeviceProperties(self.physical_device, &mut properties);
        }
        sampler_info.maxAnisotropy = properties.limits.maxSamplerAnisotropy;
        sampler_info.unnormalizedCoordinates = vk::VK_FALSE;
        sampler_info.compareOp = vk::VK_COMPARE_OP_ALWAYS;
        sampler_info.mipmapMode = vk::VK_SAMPLER_MIPMAP_MODE_LINEAR;
        sampler_info.minLod = 0.0;
        sampler_info.maxLod = vk::VK_LOD_CLAMP_NONE as _;
        sampler_info.mipLodBias = 0.0;

        unsafe {
            if vk::vkCreateSampler(self.device, &sampler_info, null(), &mut self.texture_sampler) != vk::VK_SUCCESS {
                panic!("failed to create texture sampler");
            }
        }
    }


    fn create_texture_image_view(&mut self) {
        self.texture_image_views = self.create_image_view(
            self.texture_image, 
            vk::VK_FORMAT_R8G8B8A8_SRGB, 
            vk::VK_IMAGE_ASPECT_COLOR_BIT as _,
            self.mip_levels
        );
    }

    
    fn create_image_view(
        &mut self, 
        image: vk::VkImage, 
        format: vk::VkFormat, 
        aspect_flags: vk::VkImageAspectFlags,
        mip_levels: u32
    ) -> vk::VkImageView {
        let mut image_view = vk::VkImageView::default();
        let mut view_info = vk::VkImageViewCreateInfo::default();
        view_info.sType = vk::VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
        view_info.image = image;
        view_info.viewType = vk::VK_IMAGE_VIEW_TYPE_2D;
        view_info.format = format;
        view_info.subresourceRange.aspectMask = aspect_flags;
        view_info.subresourceRange.baseMipLevel =  0;
        view_info.subresourceRange.levelCount = mip_levels;
        view_info.subresourceRange.baseArrayLayer = 0;
        view_info.subresourceRange.layerCount = 1;
        
        unsafe {
            if vk::vkCreateImageView(self.device, &view_info, null(), &mut image_view) != vk::VK_SUCCESS {
                panic!("failed to create texture image views");
            }
        }

        image_view
    }


    fn create_texture_image(&mut self) {
        let (mut width, mut height, mut channels): (i32, i32, i32) = (0, 0, 0,);
        let image = std::fs::read("assets/textures/viking_room.png").expect("failed to find file");
        let pixels = unsafe {stb_image_rust::stbi_load_from_memory(
            image.as_ptr(), 
            image.len() as _, 
            &mut width, 
            &mut height, 
            &mut channels, 
            stb_image_rust::STBI_rgb_alpha
        )};
        self.mip_levels = (std::cmp::min(width, height) as f32).log2() as u32 + 1;

        if pixels == null_mut() {
            panic!("failed to load texture image");
        }

        let image_size: vk::VkDeviceSize = (width * height * 4) as _;

        let mut staging_buffer = vk::VkBuffer::default();
        let mut staging_buffer_memory = vk::VkDeviceMemory::default();
        self.create_buffer(
            image_size, 
            vk::VK_BUFFER_USAGE_TRANSFER_SRC_BIT as _, 
            (vk::VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | vk::VK_MEMORY_PROPERTY_HOST_COHERENT_BIT) as _, 
            &mut staging_buffer, 
            &mut staging_buffer_memory
        );
        let mut data: *mut std::ffi::c_void = null_mut();
        unsafe {
            vk::vkMapMemory(self.device, staging_buffer_memory, 0, image_size, 0, &mut data);
            std::ptr::copy_nonoverlapping(pixels as _, data, image_size as _);
            vk::vkUnmapMemory(self.device, staging_buffer_memory);
            stb_image_rust::stbi_image_free(pixels);
        }

        let this = unsafe{&mut *(self as *mut Self)};
        self.create_image(
            width as _, 
            height as _, 
            self.mip_levels,
            vk::VK_SAMPLE_COUNT_1_BIT,
            vk::VK_FORMAT_R8G8B8A8_SRGB, 
            vk::VK_IMAGE_TILING_OPTIMAL, 
            vk::VK_IMAGE_USAGE_TRANSFER_SRC_BIT | vk::VK_IMAGE_USAGE_TRANSFER_DST_BIT | vk::VK_IMAGE_USAGE_SAMPLED_BIT, 
            vk::VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT as _, 
            &mut this.texture_image, 
            &mut this.texture_image_memory
        );

        self.transition_image_layout(
            self.texture_image, 
            vk::VK_FORMAT_R8G8B8A8_SRGB, 
            vk::VK_IMAGE_LAYOUT_UNDEFINED, 
            vk::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
            self.mip_levels
        );

        self.copy_buffer_to_image(staging_buffer, self.texture_image, width as _, height as _);

        unsafe {
            vk::vkDestroyBuffer(self.device, staging_buffer, null());
            vk::vkFreeMemory(self.device, staging_buffer_memory, null());
        }

        self.generate_mipmaps(self.texture_image, vk::VK_FORMAT_R8G8B8A8_SRGB, width, height, self.mip_levels);
    }


    fn generate_mipmaps(&self, image: vk::VkImage, format: vk::VkFormat, width: i32, height: i32, mip_levels: u32) {
        //the mip maps should be precomputed
        let mut format_properties = vk::VkFormatProperties::default();
        unsafe {
            vk::vkGetPhysicalDeviceFormatProperties(self.physical_device, format, &mut format_properties);
        }

        if (format_properties.optimalTilingFeatures 
            & vk::VK_FORMAT_FEATURE_SAMPLED_IMAGE_FILTER_LINEAR_BIT as u32) == 0 
        {
            panic!("texture image format does not support linear blitting");
        }


        let mut barrier = vk::VkImageMemoryBarrier::default();
        barrier.sType = vk::VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
        barrier.image = image;
        barrier.srcQueueFamilyIndex = vk::VK_QUEUE_FAMILY_IGNORED as _;
        barrier.dstQueueFamilyIndex = vk::VK_QUEUE_FAMILY_IGNORED as _;
        barrier.subresourceRange.aspectMask = vk::VK_IMAGE_ASPECT_COLOR_BIT as _;
        barrier.subresourceRange.baseArrayLayer = 0;
        barrier.subresourceRange.layerCount = 1;
        barrier.subresourceRange.levelCount = 1;

        let mut mipwidth = width;
        let mut mipheight = height;

        let command_buffer = self.begin_single_use_command_buffer(self.command_pool);

        for i in 1..self.mip_levels {
            barrier.subresourceRange.baseMipLevel = i-1;
            barrier.oldLayout = vk::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
            barrier.newLayout = vk::VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
            barrier.srcAccessMask = vk::VK_ACCESS_TRANSFER_READ_BIT as _;
            barrier.dstAccessMask = vk::VK_ACCESS_TRANSFER_WRITE_BIT as _;

            unsafe {
            vk::vkCmdPipelineBarrier(
                command_buffer, vk::VK_PIPELINE_STAGE_TRANSFER_BIT as _, vk::VK_PIPELINE_STAGE_TRANSFER_BIT as _, 0, 
                0, null(), 
                0, null(), 
                1, &barrier)
            }

            let mut blit = vk::VkImageBlit::default();
            blit.srcOffsets[0] = vk::VkOffset3D{x: 0, y:0, z:0};
            blit.srcOffsets[1] = vk::VkOffset3D{x: mipwidth, y:mipheight, z:1};
            blit.srcSubresource.aspectMask = vk::VK_IMAGE_ASPECT_COLOR_BIT as _;
            blit.srcSubresource.mipLevel = i - 1;
            blit.srcSubresource.baseArrayLayer = 0;
            blit.srcSubresource.layerCount = 1;
            blit.dstOffsets[0] = vk::VkOffset3D{x: 0, y:0, z:0};
            blit.dstOffsets[1] = vk::VkOffset3D{
                x: std::cmp::max(mipwidth / 2, 1),
                y: std::cmp::max(mipheight / 2, 1),
                z: 1
            };
            blit.dstSubresource.aspectMask = vk::VK_IMAGE_ASPECT_COLOR_BIT as _;
            blit.dstSubresource.mipLevel = i;
            blit.dstSubresource.baseArrayLayer = 0;
            blit.dstSubresource.layerCount = 1;
            unsafe {
                vk::vkCmdBlitImage(
                    command_buffer, 
                    image, vk::VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, 
                    image, vk::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, 
                    1, &blit, 
                    vk::VK_FILTER_LINEAR
                );
            }

            mipwidth = std::cmp::max(mipwidth / 2, 1);
            mipheight = std::cmp::max(mipheight / 2, 1);
            barrier.oldLayout = vk::VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
            barrier.newLayout = vk::VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
            barrier.srcAccessMask = vk::VK_ACCESS_TRANSFER_READ_BIT as _;
            barrier.dstAccessMask = vk::VK_ACCESS_SHADER_READ_BIT as _;
            unsafe {
                vk::vkCmdPipelineBarrier(
                    command_buffer, 
                    vk::VK_PIPELINE_STAGE_TRANSFER_BIT as _, vk::VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT as _, 0, 
                    0, null(), 
                    0, null(), 
                    1, &barrier
                );
            }
        }
        barrier.oldLayout = vk::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
        barrier.subresourceRange.baseMipLevel = mip_levels-1;
        unsafe {
            vk::vkCmdPipelineBarrier(
                command_buffer, 
                vk::VK_PIPELINE_STAGE_TRANSFER_BIT as _, vk::VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT as _, 0, 
                0, null(), 
                0, null(), 
                1, &barrier
            );
        }
        self.end_single_use_command_buffer(command_buffer, self.command_pool);
    }


    fn begin_single_use_command_buffer(&self, pool: vk::VkCommandPool) -> vk::VkCommandBuffer {
        let mut buf = vk::VkCommandBuffer::default();
        let mut alloc_info = vk::VkCommandBufferAllocateInfo::default();
        alloc_info.sType = vk::VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
        alloc_info.level = vk::VK_COMMAND_BUFFER_LEVEL_PRIMARY;
        alloc_info.commandPool = pool;
        alloc_info.commandBufferCount = 1;

        unsafe {
            vk::vkAllocateCommandBuffers(self.device, &alloc_info, &mut buf);
        }

        let mut begin_info = vk::VkCommandBufferBeginInfo::default();
        begin_info.sType = vk::VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
        begin_info.flags = vk::VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT as _;

        unsafe {
            vk::vkBeginCommandBuffer(buf, &begin_info);
        }

        buf
    }


    fn end_single_use_command_buffer(&self, command_buffer: vk::VkCommandBuffer, pool: vk::VkCommandPool) {
        unsafe {
            vk::vkEndCommandBuffer(command_buffer);

            let mut submit_info = vk::VkSubmitInfo::default();
            submit_info.sType = vk::VK_STRUCTURE_TYPE_SUBMIT_INFO;
            submit_info.commandBufferCount = 1;
            submit_info.pCommandBuffers = &command_buffer;

            vk::vkQueueSubmit(self.graphics_queue, 1, &submit_info, null_mut());
            vk::vkQueueWaitIdle(self.graphics_queue);

            vk::vkFreeCommandBuffers(self.device, pool, 1, &command_buffer);
        }
    }


    fn create_image(
        &self,
        width:u32, 
        height:u32, 
        mip_levels: u32,
        num_samples: vk::VkSampleCountFlagBits,
        format: vk::VkFormat, 
        tiling: vk::VkImageTiling, 
        usage: vk::VkImageUsageFlagBits, 
        properties: vk::VkMemoryPropertyFlags, 
        image: &mut vk::VkImage, 
        image_memory: &mut vk::VkDeviceMemory
    ) { 
        let mut image_info = vk::VkImageCreateInfo::default();
        image_info.sType = vk::VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
        image_info.imageType = vk::VK_IMAGE_TYPE_2D;
        image_info.extent.width = width as u32;
        image_info.extent.height = height as u32;
        image_info.extent.depth = 1;
        image_info.mipLevels = mip_levels;
        image_info.samples = num_samples;
        image_info.arrayLayers = 1;
        image_info.format = format;
        image_info.tiling = tiling;
        image_info.initialLayout = vk::VK_IMAGE_LAYOUT_UNDEFINED;
        image_info.usage = usage as u32;
        image_info.sharingMode = vk::VK_SHARING_MODE_EXCLUSIVE;
        image_info.flags = 0;


        unsafe {
            if vk::vkCreateImage(self.device, &image_info, null(), image) != vk::VK_SUCCESS {
                panic!("failed to create image");
            }

            let mut mem_requirements = vk::VkMemoryRequirements::default();
            vk::vkGetImageMemoryRequirements(self.device, *image, &mut mem_requirements);

            let mut alloc_info = vk::VkMemoryAllocateInfo::default();
            alloc_info.sType = vk::VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
            alloc_info.allocationSize = mem_requirements.size;
            alloc_info.memoryTypeIndex = self.find_memory_type(mem_requirements.memoryTypeBits, properties);

            if vk::vkAllocateMemory(self.device, &alloc_info, null(), image_memory) != vk::VK_SUCCESS {
                panic!("failed to allocate image memory");
            }

            vk::vkBindImageMemory(self.device, *image, *image_memory, 0);
        }

    }


    fn update_uniform_buffer(&mut self, current_image:usize) {
        static START_TIME: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(|| std::time::Instant::now());

        let current_time = START_TIME.elapsed().as_secs_f32();
        let mut ubo = UniformBufferObject {
            //model: math::IDENTITY,
            model: math::rot_x(current_time*45f32.to_radians()),
            view: math::get_view_mat([1.0, 1.0, 1.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            proj: math::get_proj_mat(45f32.to_radians(), WIDTH as f32 / HEIGHT as f32, 0.1, 1000.0)
        };
        ubo.proj[5] *= -1.0;

        unsafe {
            std::ptr::copy_nonoverlapping(&ubo, self.uniform_buffers_memory_mapped[current_image] as _, 1);
        }
    }


    fn create_descriptor_sets(&mut self) {
        let mut layouts = vec![self.descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
        let mut alloc_info = vk::VkDescriptorSetAllocateInfo::default();
        alloc_info.sType = vk::VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO;
        alloc_info.descriptorPool = self.descriptor_pool;
        alloc_info.descriptorSetCount = MAX_FRAMES_IN_FLIGHT as u32;
        alloc_info.pSetLayouts = layouts.as_mut_ptr();

        self.descriptor_sets.resize(MAX_FRAMES_IN_FLIGHT, vk::VkDescriptorSet::default());
        unsafe {
            if vk::vkAllocateDescriptorSets(self.device, &alloc_info, self.descriptor_sets.as_mut_ptr()) != vk::VK_SUCCESS {
                panic!("failed to allocate descriptor sets");
            }
        }

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let mut buffer_info = vk::VkDescriptorBufferInfo::default();
            buffer_info.buffer = self.uniform_buffers[i];
            buffer_info.offset = 0;
            buffer_info.range = size_of::<UniformBufferObject>() as u64;

            let mut image_info = vk::VkDescriptorImageInfo::default();
            image_info.imageLayout = vk::VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
            image_info.imageView = self.texture_image_views;
            image_info.sampler = self.texture_sampler;

            let mut descriptor_write = [vk::VkWriteDescriptorSet::default(); 2];
            descriptor_write[0].sType = vk::VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
            descriptor_write[0].dstSet = self.descriptor_sets[i];
            descriptor_write[0].dstBinding = 0;
            descriptor_write[0].dstArrayElement = 0;
            descriptor_write[0].descriptorType = vk::VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER;
            descriptor_write[0].descriptorCount = 1;
            descriptor_write[0].pBufferInfo = &buffer_info;

            descriptor_write[1].sType = vk::VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
            descriptor_write[1].dstSet = self.descriptor_sets[i];
            descriptor_write[1].dstBinding = 1;
            descriptor_write[1].dstArrayElement = 0;
            descriptor_write[1].descriptorType = vk::VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
            descriptor_write[1].descriptorCount = 1;
            descriptor_write[1].pImageInfo = &image_info;

            unsafe {
                vk::vkUpdateDescriptorSets(
                    self.device, 
                    descriptor_write.len() as u32, 
                    descriptor_write.as_ptr(), 
                    0, 
                    null()
                );
            }
        }
    }


    fn create_descriptor_pool(&mut self) {
        let mut pool_size = [vk::VkDescriptorPoolSize::default(); 2];

        pool_size[0].descriptorCount = MAX_FRAMES_IN_FLIGHT as u32;
        pool_size[0].type_ = vk::VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER;
        pool_size[1].descriptorCount = MAX_FRAMES_IN_FLIGHT as u32;
        pool_size[1].type_ = vk::VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;

        let mut pool_info = vk::VkDescriptorPoolCreateInfo::default();
        pool_info.sType = vk::VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO;
        pool_info.poolSizeCount = pool_size.len() as u32;
        pool_info.pPoolSizes = pool_size.as_ptr();
        pool_info.maxSets = MAX_FRAMES_IN_FLIGHT as u32;

        unsafe  {
            if vk::vkCreateDescriptorPool(self.device, &pool_info, null(), &mut self.descriptor_pool) != vk::VK_SUCCESS {
                panic!("failed to create descriptor pool");
            }
        }
    }


    fn create_uniform_buffers(&mut self) {
        let buffer_size = size_of::<UniformBufferObject>() as u64;
        self.uniform_buffers.resize(MAX_FRAMES_IN_FLIGHT, vk::VkBuffer::default());
        self.uniform_buffers_memory.resize(MAX_FRAMES_IN_FLIGHT, vk::VkDeviceMemory::default());
        self.uniform_buffers_memory_mapped.resize(MAX_FRAMES_IN_FLIGHT, null_mut());

        static USAGE_FLAGS: u32 = vk::VK_BUFFER_USAGE_UNIFORM_BUFFER_BIT as u32;
        static PROPERTY_FLAGS: u32 = (
            vk::VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT 
            | vk::VK_MEMORY_PROPERTY_HOST_COHERENT_BIT) as u32;

        unsafe {
            let this = &mut *(self as *mut Self);

            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.create_buffer(
                    buffer_size,
                    USAGE_FLAGS,
                    PROPERTY_FLAGS,
                    &mut this.uniform_buffers[i],
                    &mut this.uniform_buffers_memory[i],);
                vk::vkMapMemory(
                    self.device, 
                    self.uniform_buffers_memory[i], 
                    0, 
                    buffer_size, 
                    0, 
                    &mut self.uniform_buffers_memory_mapped[i]);
            }
        }
    }


    fn create_descriptor_set_layout(&mut self) {
        let mut ubo_layout_binding = vk::VkDescriptorSetLayoutBinding::default();
        ubo_layout_binding.binding = 0;
        ubo_layout_binding.descriptorType = vk::VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER;
        ubo_layout_binding.descriptorCount = 1;
        ubo_layout_binding.stageFlags = vk::VK_SHADER_STAGE_VERTEX_BIT as u32;
        ubo_layout_binding.pImmutableSamplers = null();

        let mut sampler_layout_binding = vk::VkDescriptorSetLayoutBinding::default();
        sampler_layout_binding.binding = 1;
        sampler_layout_binding.descriptorType = vk::VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
        sampler_layout_binding.descriptorCount = 1;
        sampler_layout_binding.stageFlags = vk::VK_SHADER_STAGE_FRAGMENT_BIT as u32;
        sampler_layout_binding.pImmutableSamplers = null();

        let layout_bindings = [ubo_layout_binding, sampler_layout_binding];

        let mut layout_info = vk::VkDescriptorSetLayoutCreateInfo::default();
        layout_info.sType = vk::VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO;
        layout_info.bindingCount = layout_bindings.len() as u32;
        layout_info.pBindings = layout_bindings.as_ptr();

        unsafe {
            if vk::vkCreateDescriptorSetLayout(self.device, &layout_info, null(), &mut self.descriptor_set_layout) != vk::VK_SUCCESS {
                panic!("failed to create descriptor set layout");
            }
        }
    }


    fn draw_frame(&mut self) {
        unsafe {
            vk::vkWaitForFences(self.device, 1, &self.in_flight_fences[self.current_frame], vk::VK_TRUE, u64::MAX);
            let mut image_index: u32 = 0;
            let mut result = vk::vkAcquireNextImageKHR(
                self.device,
                self.swap_chain, 
                u64::MAX, 
                self.image_available_semaphores[self.current_frame], 
                null_mut(), 
                &mut image_index);
//            if result == vk::VK_ERROR_OUT_OF_DATE_KHR {
//                self.recreate_swapchain();
//                return;
//            } else if result != vk::VK_SUCCESS && result != vk::VK_SUBOPTIMAL_KHR {
//                panic!("failed to acquire swapchain image");
//            }
            vk::vkResetFences(self.device, 1, &mut self.in_flight_fences[self.current_frame]);
            self.update_uniform_buffer(self.current_frame);
            vk::vkResetCommandBuffer(self.command_buffers[self.current_frame], 0);
            self.record_command_buffer(self.command_buffers[self.current_frame], image_index);

            let mut submit_info = vk::VkSubmitInfo::default();
            submit_info.sType = vk::VK_STRUCTURE_TYPE_SUBMIT_INFO;

            let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
            let wait_stages = [vk::VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT];
            submit_info.waitSemaphoreCount = 1;
            submit_info.pWaitSemaphores = wait_semaphores.as_ptr();
            submit_info.pWaitDstStageMask = wait_stages.as_ptr() as _;
            submit_info.commandBufferCount = 1;
            submit_info.pCommandBuffers = &self.command_buffers[self.current_frame];

            let signal_semaphores = [self.render_finished_semaphores[image_index as usize]];
            submit_info.signalSemaphoreCount = 1;
            submit_info.pSignalSemaphores = signal_semaphores.as_ptr();

            if vk::vkQueueSubmit(self.graphics_queue, 1, &submit_info, self.in_flight_fences[self.current_frame]) != vk::VK_SUCCESS {
                panic!("failed to submit to draw command buffer");
            }

            let mut present_info = vk::VkPresentInfoKHR::default();
            present_info.sType = vk::VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
            present_info.waitSemaphoreCount = 1;
            present_info.pWaitSemaphores = signal_semaphores.as_ptr();

            let swap_chains = [self.swap_chain];
            present_info.swapchainCount = 1;
            present_info.pSwapchains = swap_chains.as_ptr();
            present_info.pImageIndices = &image_index;
            present_info.pResults = null_mut();

            result = vk::vkQueuePresentKHR(self.present_queue, &present_info);
 //           if  result == vk::VK_ERROR_OUT_OF_DATE_KHR 
 //               || result == vk::VK_SUBOPTIMAL_KHR 
 //               || self.framebuffer_resized 
 //           {
 //               self.recreate_swapchain();
 //           } else if result != vk::VK_SUCCESS {
 //               panic!("failed to present swapchain image");
 //           }

            self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        }
    }


    fn create_buffer(
        &mut self, 
        size:vk::VkDeviceSize, 
        usage:vk::VkBufferUsageFlags, 
        properties: vk::VkMemoryPropertyFlags, 
        buffer: &mut vk::VkBuffer, 
        buffer_memory: &mut vk::VkDeviceMemory
    ) {
        let mut buffer_info = vk::VkBufferCreateInfo::default();
        buffer_info.sType = vk::VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
        buffer_info.size = size;
        buffer_info.usage = usage;
        buffer_info.sharingMode = vk::VK_SHARING_MODE_EXCLUSIVE;
        unsafe {
            if vk::vkCreateBuffer(self.device, &buffer_info, null(), buffer) != vk::VK_SUCCESS {
                panic!("buffer creation failed");
            }

            let mut mem_requirements = vk::VkMemoryRequirements::default();
            vk::vkGetBufferMemoryRequirements(self.device, *buffer, &mut mem_requirements);

            let mut alloc_info = vk::VkMemoryAllocateInfo::default();
            alloc_info.sType = vk::VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
            alloc_info.allocationSize = mem_requirements.size;
            alloc_info.memoryTypeIndex = self.find_memory_type(mem_requirements.memoryTypeBits, properties);

            if vk::vkAllocateMemory(self.device, &alloc_info, null_mut(), buffer_memory) != vk::VK_SUCCESS {
                panic!("failted to allocate buffer memory");
            }

            vk::vkBindBufferMemory(self.device, *buffer, *buffer_memory, 0);
        }
    }


    fn create_device_local_buffer(
        &mut self, 
        data: *const std::ffi::c_void, 
        size: u64, 
        buffer: &mut vk::VkBuffer, 
        memory: &mut vk::VkDeviceMemory,
        flags: i32
    ) {
        let mut staging_buffer = vk::VkBuffer::default();
        let mut staging_buffer_memory = vk::VkDeviceMemory::default();
        self.create_buffer(
            size, 
            vk::VK_BUFFER_USAGE_TRANSFER_SRC_BIT as _, 
            (vk::VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | vk::VK_MEMORY_PROPERTY_HOST_COHERENT_BIT) as _, 
            &mut staging_buffer, 
            &mut staging_buffer_memory);
        self.create_buffer(
            size, 
            (vk::VK_BUFFER_USAGE_TRANSFER_DST_BIT | flags) as _, 
            vk::VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT as _,
            buffer, 
            memory);

        unsafe {
            let mut data_dst: *mut std::ffi::c_void = null_mut();
            vk::vkMapMemory(self.device, staging_buffer_memory, 0, size, 0, &mut data_dst);
            std::ptr::copy_nonoverlapping(data, data_dst, size as usize);
            vk::vkUnmapMemory(self.device, staging_buffer_memory);


            self.copy_buffer(staging_buffer, *buffer, size);
            vk::vkDestroyBuffer(self.device, staging_buffer, null());
            vk::vkFreeMemory(self.device, staging_buffer_memory, null())
        }
    }


    fn create_vertex_buffer(&mut self) {
        let mut vertex_buffer = vk::VkBuffer::default();
        let mut vertex_buffer_memory = vk::VkDeviceMemory::default();
        self.create_device_local_buffer(
            self.vertices.as_ptr() as _, 
            (size_of_val(&self.vertices[0]) * self.vertices.len()) as _, 
            &mut vertex_buffer, 
            &mut vertex_buffer_memory,
            vk::VK_BUFFER_USAGE_VERTEX_BUFFER_BIT);
        self.vertex_buffer = vertex_buffer;
        self.vertex_buffer_memory = vertex_buffer_memory;
    }


    fn create_index_buffer(&mut self) {
        let mut index_buffer = vk::VkBuffer::default();
        let mut index_buffer_memory = vk::VkDeviceMemory::default();
        self.create_device_local_buffer(
            self.indices.as_ptr() as _, 
            (size_of_val(&self.indices[0]) * self.indices.len()) as _, 
            &mut index_buffer, 
            &mut index_buffer_memory,
            vk::VK_BUFFER_USAGE_INDEX_BUFFER_BIT);
        self.index_buffer = index_buffer;
        self.index_buffer_memory = index_buffer_memory;
    }


    fn copy_buffer(&self, src_buffer: vk::VkBuffer, dst_buffer: vk::VkBuffer, size: vk::VkDeviceSize) {
        let command_buffer = self.begin_single_use_command_buffer(self.transfer_command_pool);

        let mut copy_region = vk::VkBufferCopy::default();
        copy_region.size = size;
        unsafe {
            vk::vkCmdCopyBuffer(command_buffer, src_buffer, dst_buffer, 1, &copy_region);
        }

        self.end_single_use_command_buffer(command_buffer, self.transfer_command_pool);
    }


    fn transition_image_layout(
        &self, 
        image: vk::VkImage, 
        format: vk::VkFormat, 
        old_layout: vk::VkImageLayout, 
        new_layout: vk::VkImageLayout,
        mip_levels: u32
    ) {
        let command_buffer = self.begin_single_use_command_buffer(self.transfer_command_pool);

        let mut barrier = vk::VkImageMemoryBarrier::default();
        barrier.sType = vk::VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
        barrier.oldLayout = old_layout;
        barrier.newLayout = new_layout;
        barrier.srcQueueFamilyIndex = vk::VK_QUEUE_FAMILY_IGNORED as _;
        barrier.dstQueueFamilyIndex = vk::VK_QUEUE_FAMILY_IGNORED as _;
        barrier.image = image;
        barrier.subresourceRange.baseMipLevel = 0;
        barrier.subresourceRange.levelCount = mip_levels;
        barrier.subresourceRange.baseArrayLayer = 0;
        barrier.subresourceRange.layerCount = 1;
        barrier.srcAccessMask = 0;
        barrier.dstAccessMask = 0;

        let source_stage: vk::VkPipelineStageFlags;
        let destination_stage: vk::VkPipelineStageFlags;

        if new_layout == vk::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            barrier.subresourceRange.aspectMask = vk::VK_IMAGE_ASPECT_DEPTH_BIT as u32;

            if has_stencil_component(format) {
                barrier.subresourceRange.aspectMask |= vk::VK_IMAGE_ASPECT_STENCIL_BIT as u32;
            } 
        } else { 
            barrier.subresourceRange.aspectMask = vk::VK_IMAGE_ASPECT_COLOR_BIT as u32;
        }

        if old_layout == vk::VK_IMAGE_LAYOUT_UNDEFINED && new_layout == vk::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL {
            barrier.srcAccessMask = 0;
            barrier.dstAccessMask = vk::VK_ACCESS_TRANSFER_WRITE_BIT as u32;

            source_stage = vk::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as u32;
            destination_stage = vk::VK_PIPELINE_STAGE_TRANSFER_BIT as u32;
        } else if old_layout == vk::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL && new_layout == vk::VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL {
            barrier.srcAccessMask = vk::VK_ACCESS_TRANSFER_WRITE_BIT as u32;
            barrier.dstAccessMask = vk::VK_ACCESS_SHADER_READ_BIT as u32;

            source_stage = vk::VK_PIPELINE_STAGE_TRANSFER_BIT as u32;
            destination_stage = vk::VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT as u32;
        } else if old_layout == vk::VK_IMAGE_LAYOUT_UNDEFINED && new_layout  == vk::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            barrier.srcAccessMask = 0;
            barrier.dstAccessMask = (vk::VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT | vk::VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT) as u32;

            source_stage = vk::VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT as u32;
            destination_stage = vk::VK_PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT as u32;
        } else {
            panic!("unsupported layout transition");
        }

        unsafe {
            vk::vkCmdPipelineBarrier(
                command_buffer, 
                source_stage, destination_stage, 
                0, 
                0, null(), 
                0, null(), 
                1, &mut barrier
            );
        }

        self.end_single_use_command_buffer(command_buffer, self.transfer_command_pool);
    }


    fn copy_buffer_to_image(&self, buffer: vk::VkBuffer, image: vk::VkImage, width:u32, height: u32) {
        let command_buffer = self.begin_single_use_command_buffer(self.transfer_command_pool);

        let mut region = vk::VkBufferImageCopy::default();
        region.bufferOffset = 0;
        region.bufferRowLength = 0;
        region.bufferImageHeight = 0;
        region.imageSubresource.aspectMask = vk::VK_IMAGE_ASPECT_COLOR_BIT as _;
        region.imageSubresource.mipLevel = 0;
        region.imageSubresource.baseArrayLayer = 0;
        region.imageSubresource.layerCount = 1;

        region.imageOffset = vk::VkOffset3D{x: 0, y: 0, z: 0};
        region.imageExtent = vk::VkExtent3D{width, height, depth: 1};

        unsafe {
            vk::vkCmdCopyBufferToImage(
                command_buffer, 
                buffer, 
                image, 
                vk::VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, 
                1, 
                &region
            );
        }

        self.end_single_use_command_buffer(command_buffer, self.transfer_command_pool);
    }


    fn create_sync_objects(&mut self) {
        let mut semaphore_create_info = vk::VkSemaphoreCreateInfo::default();
        semaphore_create_info.sType = vk::VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
        let mut fence_create_info = vk::VkFenceCreateInfo::default();
        fence_create_info.sType = vk::VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
        fence_create_info.flags = vk::VK_FENCE_CREATE_SIGNALED_BIT as _;

        unsafe {
            self.render_finished_semaphores.resize(self.swap_chain_images.len(), null_mut());
            self.image_available_semaphores.resize(MAX_FRAMES_IN_FLIGHT, null_mut());
            self.in_flight_fences.resize(MAX_FRAMES_IN_FLIGHT, null_mut());
            for i in 0..self.swap_chain_images.len() {
                if vk::vkCreateSemaphore(
                    self.device, 
                    &semaphore_create_info, 
                    null(), 
                    &mut self.render_finished_semaphores[i]
                ) != vk::VK_SUCCESS {
                    panic!("failed to create synchronization objects");
                }
            }
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                if  vk::vkCreateSemaphore(self.device, &semaphore_create_info, null(), &mut self.image_available_semaphores[i])
                    | vk::vkCreateFence(self.device, &fence_create_info, null(), &mut self.in_flight_fences[i])
                    != vk::VK_SUCCESS
                {
                    panic!("failed to create synchronization objects");
                }
            }
        }
    }


    fn record_command_buffer(&self, command_buffer: vk::VkCommandBuffer, image_index: u32) {
        let mut begin_info = vk::VkCommandBufferBeginInfo::default();
        begin_info.sType = vk::VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
        begin_info.flags = 0;
        begin_info.pInheritanceInfo = null();

        if unsafe{vk::vkBeginCommandBuffer(command_buffer, &begin_info)} != vk::VK_SUCCESS {
            panic!("failed to begin recording command buffer");
        }
        let mut render_pass_info = vk::VkRenderPassBeginInfo::default();
        render_pass_info.sType = vk::VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO;
        render_pass_info.renderPass = self.render_pass;
        render_pass_info.framebuffer = self.swap_chain_framebuffers[image_index as usize];
        render_pass_info.renderArea.offset = vk::VkOffset2D{x: 0, y: 0};
        render_pass_info.renderArea.extent = self.swap_chain_extent;
        let mut clear_values = [vk::VkClearValue::default(); 2];
        clear_values[0].color = vk::VkClearColorValue{float32: [0.0, 0.0, 0.0, 1.0]};
        clear_values[1].depthStencil = vk::VkClearDepthStencilValue{depth: 1.0, stencil: 0};
        render_pass_info.clearValueCount = clear_values.len() as u32;
        render_pass_info.pClearValues = clear_values.as_ptr();

        unsafe{
            vk::vkCmdBeginRenderPass(command_buffer, &render_pass_info, vk::VK_SUBPASS_CONTENTS_INLINE);
            vk::vkCmdBindPipeline(command_buffer, vk::VK_PIPELINE_BIND_POINT_GRAPHICS, self.pipeline);

            let mut viewport = vk::VkViewport::default();
            viewport.x = 0.0;
            viewport.y = 0.0;
            viewport.width = self.swap_chain_extent.width as _;
            viewport.height = self.swap_chain_extent.height as _;
            viewport.minDepth = 0.0;
            viewport.maxDepth = 1.0; 
            vk::vkCmdSetViewport(command_buffer, 0, 1, &viewport);

            let mut scissor = vk::VkRect2D::default();
            scissor.offset = vk::VkOffset2D{x:0, y:0};
            scissor.extent = self.swap_chain_extent;
            vk::vkCmdSetScissor(command_buffer, 0, 1, &scissor);

            let vertex_buffers = [self.vertex_buffer];
            let offsets = [0 as vk::VkDeviceSize];
            vk::vkCmdBindVertexBuffers(command_buffer, 0, 1, vertex_buffers.as_ptr(), offsets.as_ptr());
            vk::vkCmdBindIndexBuffer(command_buffer, self.index_buffer, 0, vk::VK_INDEX_TYPE_UINT32);

            vk::vkCmdBindDescriptorSets(
                command_buffer, 
                vk::VK_PIPELINE_BIND_POINT_GRAPHICS, 
                self.pipeline_layout, 
                0, 
                1, 
                &self.descriptor_sets[self.current_frame], 
                0, 
                null());
            vk::vkCmdDrawIndexed(command_buffer, self.indices.len() as _, 1, 0, 0, 0);
            vk::vkCmdEndRenderPass(command_buffer);

            if vk::vkEndCommandBuffer(command_buffer) != vk::VK_SUCCESS {
                panic!("failed to record command buffer");
            }
        }
    }


    fn create_command_buffers(&mut self) {
        let mut alloc_info = vk::VkCommandBufferAllocateInfo::default();
        alloc_info.sType = vk::VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
        alloc_info.commandPool = self.command_pool;
        alloc_info.level = vk::VK_COMMAND_BUFFER_LEVEL_PRIMARY;
        alloc_info.commandBufferCount = 1;

        self.command_buffers.resize(MAX_FRAMES_IN_FLIGHT, null_mut());
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            if unsafe{vk::vkAllocateCommandBuffers(
                self.device, 
                &alloc_info, 
                &mut self.command_buffers[i]
            )} != vk::VK_SUCCESS {
                panic!("failed to allocate command buffers");
            }
        }
    }


    fn create_command_pools(&mut self) {
        let queue_families = self.get_queue_families(self.physical_device);

        let mut graphics_pool_info = vk::VkCommandPoolCreateInfo::default();
        graphics_pool_info.sType = vk::VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
        graphics_pool_info.flags = vk::VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT as _;
        graphics_pool_info.queueFamilyIndex = queue_families.graphics_family.unwrap();

        let mut transfer_pool = vk::VkCommandPoolCreateInfo::default();
        transfer_pool.sType = vk::VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
        transfer_pool.flags = vk::VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT as _;
        transfer_pool.queueFamilyIndex = queue_families.transfer_family.unwrap();

        if unsafe{vk::vkCreateCommandPool(self.device, &graphics_pool_info, null(), &mut self.command_pool)} != vk::VK_SUCCESS {
            panic!("failed to create command pool");
        }

        if unsafe{vk::vkCreateCommandPool(self.device, &transfer_pool, null(), &mut self.transfer_command_pool)} != vk::VK_SUCCESS {
            panic!("failed to create command pool");
        }
    }


    fn create_framebuffers(&mut self) {
        self.swap_chain_framebuffers.resize(self.swap_chain_image_views.len(), vk::VkFramebuffer::default());
        for i in 0..self.swap_chain_image_views.len() {
            let attachments = [self.color_image_view, self.depth_view, self.swap_chain_image_views[i]];

            let mut framebuffer_create_info = vk::VkFramebufferCreateInfo::default();
            framebuffer_create_info.sType = vk::VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO;
            framebuffer_create_info.renderPass = self.render_pass;
            framebuffer_create_info.attachmentCount = attachments.len() as u32;
            framebuffer_create_info.pAttachments = attachments.as_ptr();
            framebuffer_create_info.width = self.swap_chain_extent.width;
            framebuffer_create_info.height = self.swap_chain_extent.height;
            framebuffer_create_info.layers = 1;

            if unsafe{vk::vkCreateFramebuffer(
                self.device, 
                &framebuffer_create_info, 
                null(), 
                &mut self.swap_chain_framebuffers[i]
            )} != vk::VK_SUCCESS {
                panic!("failed to create framebuffer");
            }
        }
    }


    fn create_render_pass(&mut self) {
        let mut depth_attachment = vk::VkAttachmentDescription::default();
        depth_attachment.format = self.find_depth_format();
        depth_attachment.samples = self.msaa_samples;
        depth_attachment.loadOp = vk::VK_ATTACHMENT_LOAD_OP_CLEAR;
        depth_attachment.storeOp = vk::VK_ATTACHMENT_STORE_OP_DONT_CARE;
        depth_attachment.stencilLoadOp = vk::VK_ATTACHMENT_LOAD_OP_DONT_CARE;
        depth_attachment.stencilStoreOp = vk::VK_ATTACHMENT_STORE_OP_DONT_CARE;
        depth_attachment.initialLayout = vk::VK_IMAGE_LAYOUT_UNDEFINED;
        depth_attachment.finalLayout = vk::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL;

        let mut depth_attachment_ref = vk::VkAttachmentReference::default();
        depth_attachment_ref.attachment = 1;
        depth_attachment_ref.layout = vk::VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL;

        let mut color_attachment = vk::VkAttachmentDescription::default();
        color_attachment.format = self.swap_chain_image_format;
        color_attachment.samples = self.msaa_samples;
        color_attachment.loadOp = vk::VK_ATTACHMENT_LOAD_OP_CLEAR;
        color_attachment.storeOp = vk::VK_ATTACHMENT_STORE_OP_STORE;
        color_attachment.stencilLoadOp = vk::VK_ATTACHMENT_LOAD_OP_DONT_CARE;
        color_attachment.stencilStoreOp = vk::VK_ATTACHMENT_STORE_OP_DONT_CARE;
        color_attachment.initialLayout = vk::VK_IMAGE_LAYOUT_UNDEFINED;
        color_attachment.finalLayout = vk::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

        let mut color_attachment_ref = vk::VkAttachmentReference::default();
        color_attachment_ref.attachment = 0;
        color_attachment_ref.layout = vk::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

        let mut color_attachment_resolve = vk::VkAttachmentDescription::default();
        color_attachment_resolve.format = self.swap_chain_image_format;
        color_attachment_resolve.samples = vk::VK_SAMPLE_COUNT_1_BIT;
        color_attachment_resolve.loadOp = vk::VK_ATTACHMENT_LOAD_OP_DONT_CARE;
        color_attachment_resolve.storeOp = vk::VK_ATTACHMENT_STORE_OP_STORE;
        color_attachment_resolve.stencilLoadOp = vk::VK_ATTACHMENT_LOAD_OP_DONT_CARE;
        color_attachment_resolve.stencilStoreOp = vk::VK_ATTACHMENT_STORE_OP_DONT_CARE;
        color_attachment_resolve.initialLayout = vk::VK_IMAGE_LAYOUT_UNDEFINED;
        color_attachment_resolve.finalLayout = vk::VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;

        let mut color_attachment_resolve_ref = vk::VkAttachmentReference::default();
        color_attachment_resolve_ref.attachment = 2;
        color_attachment_resolve_ref.layout = vk::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

        let mut subpass = vk::VkSubpassDescription::default();
        subpass.pipelineBindPoint = vk::VK_PIPELINE_BIND_POINT_GRAPHICS;
        subpass.colorAttachmentCount = 1;
        subpass.pColorAttachments = &color_attachment_ref;
        subpass.pDepthStencilAttachment = &depth_attachment_ref;
        subpass.pResolveAttachments = &color_attachment_resolve_ref;

        let attachments = [color_attachment, depth_attachment, color_attachment_resolve];

        let mut render_pass_info = vk::VkRenderPassCreateInfo::default();
        render_pass_info.sType = vk::VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO;
        render_pass_info.attachmentCount = attachments.len() as u32;
        render_pass_info.pAttachments = attachments.as_ptr();
        render_pass_info.subpassCount = 1;
        render_pass_info.pSubpasses = &subpass;

        let mut dependency = vk::VkSubpassDependency::default();
        dependency.srcSubpass = vk::VK_SUBPASS_EXTERNAL as _;
        dependency.dstSubpass = 0;
        dependency.srcStageMask = (vk::VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT | vk::VK_PIPELINE_STAGE_LATE_FRAGMENT_TESTS_BIT) as u32;
        dependency.srcAccessMask = (vk::VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT | vk::VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT) as u32;
        dependency.dstStageMask = (vk::VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT | vk::VK_PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT) as u32;
        dependency.dstAccessMask = (vk::VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT | vk::VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT) as u32;

        render_pass_info.dependencyCount = 1;
        render_pass_info.pDependencies = &dependency;

        if unsafe{vk::vkCreateRenderPass(
            self.device, 
            &render_pass_info, 
            null(), 
            &mut self.render_pass
        )} != vk::VK_SUCCESS {
            panic!("rander pass creation failed")
        }
    }


    fn create_graphics_pipeline(&mut self) {
        let vertex_shader_code = read_file("./shaders/vert.spv");
        let fragment_shader_code = read_file("./shaders/frag.spv");

        let vert_shader_module = self.create_shader_module(&vertex_shader_code);
        let frag_shader_module = self.create_shader_module(&fragment_shader_code);

        let mut vert_shader_stage_info = vk::VkPipelineShaderStageCreateInfo::default();
        vert_shader_stage_info.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
        vert_shader_stage_info.stage = vk::VK_SHADER_STAGE_VERTEX_BIT;
        vert_shader_stage_info.module = vert_shader_module;
        vert_shader_stage_info.pName = "main\0".as_ptr() as _;

        let mut frag_shader_stage_info = vk::VkPipelineShaderStageCreateInfo::default();
        frag_shader_stage_info.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
        frag_shader_stage_info.stage = vk::VK_SHADER_STAGE_FRAGMENT_BIT;
        frag_shader_stage_info.module = frag_shader_module;
        frag_shader_stage_info.pName = "main\0".as_ptr() as _;

        let shader_stages = [vert_shader_stage_info, frag_shader_stage_info];

        let dynamic_states = [vk::VK_DYNAMIC_STATE_VIEWPORT, vk::VK_DYNAMIC_STATE_SCISSOR];

        let mut dynamic_state = vk::VkPipelineDynamicStateCreateInfo::default();
        dynamic_state.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO;
        dynamic_state.dynamicStateCount = dynamic_states.len() as _;
        dynamic_state.pDynamicStates = dynamic_states.as_ptr();

        let mut vertex_input_info = vk::VkPipelineVertexInputStateCreateInfo::default();
        let binding_description = Vertex::get_binding_description();
        let attribute_descriptions = Vertex::get_attribute_descriptions();
        vertex_input_info.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO;
        vertex_input_info.vertexBindingDescriptionCount = 1;
        vertex_input_info.pVertexBindingDescriptions = &binding_description;
        vertex_input_info.vertexAttributeDescriptionCount = attribute_descriptions.len() as _;
        vertex_input_info.pVertexAttributeDescriptions = attribute_descriptions.as_ptr();

        let mut input_assembly = vk::VkPipelineInputAssemblyStateCreateInfo::default();
        input_assembly.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO;
        input_assembly.topology = vk::VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST;
        input_assembly.primitiveRestartEnable = vk::VK_FALSE;

        let mut viewport_state = vk::VkPipelineViewportStateCreateInfo::default();
        viewport_state.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO;
        viewport_state.viewportCount = 1;
        viewport_state.scissorCount = 1;

        let mut rasterizer = vk::VkPipelineRasterizationStateCreateInfo::default();
        rasterizer.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO;
        rasterizer.rasterizerDiscardEnable = vk::VK_FALSE;
        rasterizer.polygonMode = vk::VK_POLYGON_MODE_FILL;
        rasterizer.lineWidth = 1.0;
        rasterizer.cullMode = vk::VK_CULL_MODE_BACK_BIT as _;
        rasterizer.frontFace = vk::VK_FRONT_FACE_COUNTER_CLOCKWISE;
        rasterizer.depthBiasEnable = vk::VK_FALSE;
        rasterizer.depthBiasConstantFactor = 0.0;
        rasterizer.depthBiasClamp = 0.0;
        rasterizer.depthBiasSlopeFactor = 0.0;

        let mut multisampling = vk::VkPipelineMultisampleStateCreateInfo::default();
        multisampling.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO;
        multisampling.sampleShadingEnable = vk::VK_FALSE;
        multisampling.rasterizationSamples = self.msaa_samples;
        multisampling.minSampleShading = 1.0;
        multisampling.pSampleMask = null();
        multisampling.alphaToCoverageEnable = vk::VK_FALSE;
        multisampling.alphaToOneEnable = vk::VK_FALSE;
        //fragment shader runs per sample not per fragment
        multisampling.sampleShadingEnable = vk::VK_TRUE;
        multisampling.minSampleShading = 0.2;

        let mut color_blend_attachment = vk::VkPipelineColorBlendAttachmentState::default();
        color_blend_attachment.colorWriteMask = (
            vk::VK_COLOR_COMPONENT_R_BIT | vk::VK_COLOR_COMPONENT_G_BIT | 
            vk::VK_COLOR_COMPONENT_B_BIT | vk::VK_COLOR_COMPONENT_A_BIT) as u32;
        color_blend_attachment.blendEnable = vk::VK_TRUE;
        color_blend_attachment.srcColorBlendFactor = vk::VK_BLEND_FACTOR_SRC_ALPHA;
        color_blend_attachment.dstColorBlendFactor = vk::VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA;
        color_blend_attachment.colorBlendOp = vk::VK_BLEND_OP_ADD;
        color_blend_attachment.srcColorBlendFactor = vk::VK_BLEND_FACTOR_ONE;
        color_blend_attachment.srcAlphaBlendFactor = vk::VK_BLEND_FACTOR_ZERO;
        color_blend_attachment.alphaBlendOp = vk::VK_BLEND_OP_ADD;

        let mut color_blend = vk::VkPipelineColorBlendStateCreateInfo::default();
        color_blend.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO;
        color_blend.logicOpEnable = vk::VK_FALSE;
        color_blend.logicOp = vk::VK_LOGIC_OP_COPY;
        color_blend.attachmentCount = 1;
        color_blend.pAttachments = &color_blend_attachment;

        let mut pipeline_layout_create_info = vk::VkPipelineLayoutCreateInfo::default();
        pipeline_layout_create_info.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
        pipeline_layout_create_info.setLayoutCount = 1;
        pipeline_layout_create_info.pSetLayouts = &self.descriptor_set_layout;
        pipeline_layout_create_info.pushConstantRangeCount = 0;
        pipeline_layout_create_info.pPushConstantRanges = null();

        if unsafe{vk::vkCreatePipelineLayout(
            self.device, 
            &pipeline_layout_create_info, 
            null(), 
            &mut 
            self.pipeline_layout
        )} != vk::VK_SUCCESS {
            panic!("failed to create pipeline layout");
        }

        let mut depth_stencil = vk::VkPipelineDepthStencilStateCreateInfo::default();
        depth_stencil.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO;
        depth_stencil.depthTestEnable = vk::VK_TRUE;
        depth_stencil.depthWriteEnable = vk::VK_TRUE;
        depth_stencil.depthCompareOp = vk::VK_COMPARE_OP_LESS;
        depth_stencil.depthBoundsTestEnable = vk::VK_FALSE;
        depth_stencil.minDepthBounds = 0.0;
        depth_stencil.maxDepthBounds = 1.0;
        depth_stencil.stencilTestEnable = vk::VK_FALSE;

        let mut pipeline_info = vk::VkGraphicsPipelineCreateInfo::default();
        pipeline_info.sType = vk::VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
        pipeline_info.stageCount = 2;
        pipeline_info.pStages = shader_stages.as_ptr();
        pipeline_info.pVertexInputState = &vertex_input_info;
        pipeline_info.pInputAssemblyState = &input_assembly;
        pipeline_info.pViewportState = &viewport_state;
        pipeline_info.pRasterizationState = &rasterizer;
        pipeline_info.pMultisampleState = &multisampling;
        pipeline_info.pDepthStencilState = &depth_stencil;
        pipeline_info.pColorBlendState = &color_blend;
        pipeline_info.pDynamicState = &dynamic_state;
        pipeline_info.layout = self.pipeline_layout;
        pipeline_info.renderPass = self.render_pass;
        pipeline_info.subpass = 0;
        pipeline_info.basePipelineHandle = null_mut();
        pipeline_info.basePipelineIndex = -1;

        if unsafe{vk::vkCreateGraphicsPipelines(
            self.device, 
            null_mut(), 
            1, 
            &pipeline_info, 
            null(), 
            &mut self.pipeline
        ) != vk::VK_SUCCESS} {
            panic!("pipeline creation failed");
        }

        unsafe {
            vk::vkDestroyShaderModule(self.device, vert_shader_module, null());
            vk::vkDestroyShaderModule(self.device, frag_shader_module, null());
        }
    }
    

    fn create_image_views(&mut self) {
        self.swap_chain_image_views.resize(self.swap_chain_images.len(), vk::VkImageView::default());

        for i in 0..self.swap_chain_image_views.len() {
            self.swap_chain_image_views[i] = self.create_image_view(
                self.swap_chain_images[i], 
                self.swap_chain_image_format,
                vk::VK_IMAGE_ASPECT_COLOR_BIT as _,
                1
            );
        }
    }


    fn create_swapchain(&mut self) {
        let swap_chain_support = self.query_swap_chain_support(self.physical_device);

        let surface_format = self.choose_swap_chain_format(&swap_chain_support.formats);
        let present_mode = self.choose_swapchain_present_mode(&swap_chain_support.present_modes);
        let extent = self.choose_swap_extent(&swap_chain_support.capabilities);
        let mut image_count:u32 = swap_chain_support.capabilities.minImageCount + 1;
        let max_image_count = swap_chain_support.capabilities.maxImageCount;
        if max_image_count > 0 && image_count > max_image_count {
            image_count = max_image_count;
        }

        let mut create_info = vk::VkSwapchainCreateInfoKHR::default();
        create_info.sType = vk::VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
        create_info.surface = self.surface;
        create_info.minImageCount = image_count;
        create_info.imageColorSpace = surface_format.colorSpace;
        create_info.imageExtent = extent;
        create_info.imageArrayLayers = 1;
        create_info.imageUsage = vk::VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT as _;
        create_info.imageFormat = surface_format.format;

        let indices = self.get_queue_families(self.physical_device);
        let queue_family_indices: [u32; 3] = [
            indices.graphics_family.unwrap(), 
            indices.present_family.unwrap(), 
            indices.transfer_family.unwrap()
        ];

        let unique_families: Vec<_> = HashSet::from(queue_family_indices).into_iter().collect();

        if unique_families.len() > 1 {
            create_info.imageSharingMode = vk::VK_SHARING_MODE_CONCURRENT;
            create_info.queueFamilyIndexCount = unique_families.len() as u32;
            create_info.pQueueFamilyIndices = unique_families.as_ptr();
        } else {
            create_info.imageSharingMode = vk::VK_SHARING_MODE_EXCLUSIVE;
            create_info.queueFamilyIndexCount = 0;
            create_info.pQueueFamilyIndices = null();
        }
        
        create_info.preTransform = swap_chain_support.capabilities.currentTransform;
        create_info.compositeAlpha = vk::VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
        create_info.presentMode = present_mode;
        create_info.clipped = vk::VK_TRUE;
        create_info.oldSwapchain = null_mut();

        if unsafe{vk::vkCreateSwapchainKHR(
            self.device, 
            &mut create_info, 
            null(), 
            &mut self.swap_chain
        )} != vk::VK_SUCCESS {
            panic!("swapchain generation failed");
        }

        unsafe {
            vk::vkGetSwapchainImagesKHR(self.device, self.swap_chain, &mut image_count, null_mut());
            self.swap_chain_images.resize(image_count as usize, vk::VkImage::default());
            vk::vkGetSwapchainImagesKHR(self.device, self.swap_chain, &mut image_count, self.swap_chain_images.as_mut_ptr());
            self.swap_chain_image_format = surface_format.format;
            self.swap_chain_extent = extent;
        }
    }


    fn recreate_swapchain(&mut self) {
        unsafe {
            let (mut width, mut height): (i32, i32 ) = (0,0);
            glfwGetFramebufferSize(self.window, &mut width, &mut height);
            while width == 0 || height == 0 {
                glfwGetFramebufferSize(self.window, &mut width, &mut height);
                glfwWaitEvents();
            }

            vk::vkDeviceWaitIdle(self.device);
            self.cleanup_swapchain();

            self.create_swapchain();
            self.create_image_views();
            self.create_color_resources();
            self.create_depth_resources();
            self.create_framebuffers();
        }
    }


    fn cleanup_swapchain(&mut self) {
        unsafe {
            for framebuffer in self.swap_chain_framebuffers.iter() {
                vk::vkDestroyFramebuffer(self.device, *framebuffer, null());
            }

            for image_view in self.swap_chain_image_views.iter() {
                vk::vkDestroyImageView(self.device, *image_view, null());
            }

            vk::vkDestroyImageView(self.device, self.color_image_view, null());
            vk::vkDestroyImage(self.device, self.color_image, null());
            vk::vkFreeMemory(self.device, self.color_image_memory, null());

            vk::vkDestroyImageView(self.device, self.depth_view, null());
            vk::vkDestroyImage(self.device, self.depth_image, null());
            vk::vkFreeMemory(self.device, self.depth_image_memory, null());
            
            vk::vkDestroySwapchainKHR(self.device, self.swap_chain, null());
        }
    }


    fn create_surface(&mut self) {
        unsafe {
            if glfwCreateWindowSurface(
                self.instance as _, 
                self.window, 
                null(), 
                &mut self.surface as *mut _ as _
            ) != vk::VK_SUCCESS {
                panic!("failed to create surface");
            }
        }
    }


    fn create_logical_device(&mut self) {
        let indices = self.get_queue_families(self.physical_device);

        let mut queue_create_infos:Vec<vk::VkDeviceQueueCreateInfo> = Vec::new();
        let unique_queue_families = HashSet::from(
            [indices.graphics_family.unwrap(), indices.present_family.unwrap(), indices.transfer_family.unwrap()]
        );

        for family in unique_queue_families {
            let mut queue_create_info = vk::VkDeviceQueueCreateInfo::default();
            queue_create_info.sType = vk::VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
            queue_create_info.queueFamilyIndex = family;
            queue_create_info.queueCount = 1;
            let queue_priority:f32 = 1.0;
            queue_create_info.pQueuePriorities = &queue_priority;
            queue_create_infos.push(queue_create_info);
        }

        let mut physical_device_features = vk::VkPhysicalDeviceFeatures::default();
        physical_device_features.samplerAnisotropy = vk::VK_TRUE;
        //fragment shader runs per sample not per fragment
        physical_device_features.sampleRateShading = vk::VK_TRUE;

        let mut create_info = vk::VkDeviceCreateInfo::default();
        create_info.sType = vk::VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
        create_info.pQueueCreateInfos = queue_create_infos.as_ptr();
        create_info.queueCreateInfoCount = queue_create_infos.len() as u32;
        create_info.pEnabledFeatures = &physical_device_features;
        create_info.enabledExtensionCount = DEVICE_EXTENSIONS.len() as _;
        create_info.ppEnabledExtensionNames = DEVICE_EXTENSIONS.as_ptr() as _;

        if ENABLE_VALIDATION_LAYERS {
            create_info.enabledLayerCount = VALIDATION_LAYERS.len() as u32;
            create_info.ppEnabledLayerNames = VALIDATION_LAYERS.as_ptr() as _;
        } else {
            create_info.enabledLayerCount = 0;
        }

        if unsafe{vk::vkCreateDevice(self.physical_device, &mut create_info, null(), &mut self.device)} != vk::VK_SUCCESS {
            panic!("could not initialize logical device");
        }

        unsafe{
            vk::vkGetDeviceQueue(self.device, indices.graphics_family.unwrap(), 0, &mut self.graphics_queue);
            vk::vkGetDeviceQueue(self.device, indices.present_family.unwrap(), 0, &mut self.present_queue);
            vk::vkGetDeviceQueue(self.device, indices.transfer_family.unwrap(), 0, &mut self.transfer_queue);
        }

    }


    fn pick_physical_device(&mut self) {
        unsafe {
            let mut device_count:u32 = 0;
            vk::vkEnumeratePhysicalDevices(self.instance, &mut device_count, null_mut());
            if device_count == 0 {
                panic!("failed to find gpus with vulkan support");
            }
            let mut physical_devices: Vec<vk::VkPhysicalDevice> = Vec::with_capacity(device_count as usize);
            vk::vkEnumeratePhysicalDevices(self.instance, &mut device_count, physical_devices.as_mut_ptr());
            physical_devices.set_len(device_count as usize);

            let mut rankings = BTreeMap::new();

            for device in physical_devices {
                let ranking = self.rate_device_suitability(device);
                rankings.insert(ranking, device);
            }

            let final_score;
            (final_score, self.physical_device) = rankings.last_entry().unwrap().remove_entry();
            self.msaa_samples = self.get_max_usable_sample_count();

            if final_score == 0 {
                panic!("failed to find a suitable gpu");
            }
        }
    }


    fn find_memory_type(&self, type_filter:u32, properties: vk::VkMemoryPropertyFlags) -> u32 {
        let mut mem_properties = vk::VkPhysicalDeviceMemoryProperties::default();
        unsafe{vk::vkGetPhysicalDeviceMemoryProperties(
            self.physical_device, 
            &mut mem_properties
        )};
        
        for i in 0..mem_properties.memoryTypeCount {
            if  type_filter & (i << 1) != 0 
                &&  mem_properties.memoryTypes[i as usize].propertyFlags 
                    & properties 
                == properties 
            {
                return i;
            }
        }

        panic!("failed to find suitable memory type");
    }


    fn setup_debug_messanger(&mut self) {
        if !ENABLE_VALIDATION_LAYERS {return}

        let mut create_info = vk::VkDebugUtilsMessengerCreateInfoEXT::default();
        populate_debug_messenger_create_info(&mut create_info);

        if create_debug_utils_messenger(self.instance, &create_info, null(), &mut self.debug_messenger) != vk::VK_SUCCESS {
            panic!("could not set up debug messenger")
        }
    }


    fn check_validation_layer_support(&self) -> bool {
        unsafe {
            let mut layer_count:u32 = 0;
            vk::vkEnumerateInstanceLayerProperties(&mut layer_count, null_mut());
            let mut available_layers: Vec<vk::VkLayerProperties> = Vec::with_capacity(layer_count as usize);
            vk::vkEnumerateInstanceLayerProperties(&mut layer_count, available_layers.as_mut_ptr());
            available_layers.set_len(layer_count as usize);

            for layer in VALIDATION_LAYERS {
                let mut layer_found = false;
                for layer_properties in available_layers.iter() {
                    let s = std::ffi::CStr::from_ptr(layer_properties.layerName.as_ptr()).to_str().unwrap();
                    if s == layer{
                        layer_found = true;
                    }
                }
                if !layer_found {return false}
            }
        }

        true
    }

    fn get_required_extensions(&self) -> Vec<*const i8> {
        unsafe {
            let mut glfw_extension_count:u32 = 0;
            let glfw_extensions = glfwGetRequiredInstanceExtensions(&mut glfw_extension_count);
            let extensions_slice = std::slice::from_raw_parts(glfw_extensions, glfw_extension_count as usize);
            let mut extensions = extensions_slice.to_vec();

            if ENABLE_VALIDATION_LAYERS {
                extensions.push(vk::VK_EXT_DEBUG_UTILS_EXTENSION_NAME.as_ptr() as _);
            }

            extensions
        }
    }


    fn create_instance(&mut self) {
        if ENABLE_VALIDATION_LAYERS && !self.check_validation_layer_support() {
            panic!("validation layers requested, but not available");
        }

        let mut appinfo = vk::VkApplicationInfo::default();
        appinfo.sType = vk::VK_STRUCTURE_TYPE_APPLICATION_INFO;
        appinfo.pApplicationName = "Hello Triangle\0".as_ptr() as _;
        appinfo.applicationVersion = 0;
        appinfo.pEngineName = "No Engine\0".as_ptr() as _;
        appinfo.engineVersion = get_api_version(1,0);
        appinfo.apiVersion = get_api_version(1,0); 

        let mut create_info = vk::VkInstanceCreateInfo::default();
        create_info.sType = vk::VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
        create_info.pApplicationInfo = &appinfo;

        let mut debug_create_info = vk::VkDebugUtilsMessengerCreateInfoEXT::default();
        if ENABLE_VALIDATION_LAYERS {
            create_info.enabledLayerCount = VALIDATION_LAYERS.len() as _;
            create_info.ppEnabledLayerNames = VALIDATION_LAYERS.as_ptr() as _;

            populate_debug_messenger_create_info(&mut debug_create_info);
            create_info.pNext = &debug_create_info as *const _ as _;

        } else {
            create_info.enabledLayerCount = 0;
            create_info.pNext = null();
        }

        let extensions = self.get_required_extensions();
        create_info.enabledExtensionCount = extensions.len() as _;
        create_info.ppEnabledExtensionNames = extensions.as_ptr();

        //unsafe {
        //    let mut extension_count:u32 = 0;
        //    vk::vkEnumerateInstanceExtensionProperties(null(), &mut extension_count, null_mut());
        //    let mut extensions: Vec<vk::VkExtensionProperties> = Vec::with_capacity(extension_count as usize);
        //    vk::vkEnumerateInstanceExtensionProperties(null(), &mut extension_count, extensions.as_mut_ptr());
        //    extensions.set_len(extension_count as usize);
        //    for extension in extensions {
        //        println!("{:?}", std::ffi::CStr::from_ptr(extension.extensionName.as_ptr()));
        //    }
        //}

        let res: vk::VkResult = unsafe{vk::vkCreateInstance(&create_info, null(), &mut self.instance)};
        if res != vk::VK_SUCCESS {
            panic!("failed to create instance!");
        }
    }

    pub fn get_queue_families(&self, device: vk::VkPhysicalDevice) -> QueueFamilyIndices {
        let mut indices = QueueFamilyIndices::default();

        unsafe {
            let mut queue_family_count:u32 = 0;
            vk::vkGetPhysicalDeviceQueueFamilyProperties(device, &mut queue_family_count, null_mut());
            let mut queue_families: Vec<vk::VkQueueFamilyProperties> = Vec::with_capacity(queue_family_count as usize);
            vk::vkGetPhysicalDeviceQueueFamilyProperties(device, &mut queue_family_count, queue_families.as_mut_ptr());
            queue_families.set_len(queue_family_count as usize);

            let mut present_support: vk::VkBool32 = vk::VK_FALSE;
            let mut graphics_support: bool;
            let mut transfer_support: bool;

            for (i, &family_properties) in queue_families.iter().enumerate() {
                graphics_support = family_properties.queueFlags & vk::VK_QUEUE_GRAPHICS_BIT as u32 != 0;
                transfer_support = family_properties.queueFlags & vk::VK_QUEUE_TRANSFER_BIT as u32 != 0;
                vk::vkGetPhysicalDeviceSurfaceSupportKHR(
                    device, 
                    i as u32, 
                    self.surface, 
                    &mut present_support
                );

                if indices.transfer_family.is_none() 
                    && transfer_support
                {
                    indices.transfer_family = Some(i as u32);
                }

                if indices.graphics_family.is_none() 
                    && graphics_support 
                    && Some(i as u32) != indices.transfer_family 
                {
                    indices.graphics_family = Some(i as u32);
                }

                if indices.present_family.is_none() 
                    && present_support == vk::VK_TRUE 
                    && Some(i as u32) != indices.transfer_family 
                {
                    indices.present_family = Some(i as u32);
                }
            }
        }

        if let None = indices.graphics_family && let Some(n) = indices.transfer_family {
            indices.graphics_family = Some(n);
        }

        if let None = indices.present_family && let Some(n) = indices.transfer_family {
            indices.present_family = Some(n);
        }

        indices
    }


    fn rate_device_suitability(&self, device: vk::VkPhysicalDevice) -> i32 {
        let mut score: i32 = 0;
        let mut device_properties = vk::VkPhysicalDeviceProperties::default();
        let mut device_features = vk::VkPhysicalDeviceFeatures::default();
        unsafe {
            vk::vkGetPhysicalDeviceProperties(device, &mut device_properties);
            vk::vkGetPhysicalDeviceFeatures(device, &mut device_features);
        }

        if device_properties.deviceType == vk::VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU {
            score += 1000;
        }

        score += device_properties.limits.maxImageDimension2D as i32;

        if device_features.geometryShader == 0 {
            eprintln!("failed to find geometry shader support");
            return 0;
        }

        if !self.get_queue_families(device).is_complete() {
            return 0;
        }

        if !self.check_device_extension_support(device) {
            return 0;
        }

        if device_features.samplerAnisotropy == vk::VK_FALSE {
            return 0;
        }

        let swap_chain_support_details = self.query_swap_chain_support(device);
        let swap_chain_supported = !swap_chain_support_details.formats.is_empty() && !swap_chain_support_details.present_modes.is_empty();
        if !swap_chain_supported {
            return 0;
        }

        score
    }


    fn check_device_extension_support(&self, device: vk::VkPhysicalDevice) -> bool {
        unsafe {
            let mut extension_count: u32 = 0;
            vk::vkEnumerateDeviceExtensionProperties(device, null(), &mut extension_count, null_mut());
            let mut available_extensions: Vec<vk::VkExtensionProperties> = Vec::with_capacity(extension_count as usize);
            vk::vkEnumerateDeviceExtensionProperties(device, null(), &mut extension_count, available_extensions.as_mut_ptr());
            available_extensions.set_len(extension_count as usize);

            let mut required_extensions: HashSet<*const u8> = HashSet::from(DEVICE_EXTENSIONS);
            for extension in available_extensions {
                required_extensions.remove(&(extension.extensionName.as_ptr() as _));
            }

            !required_extensions.is_empty()
        }
    }


    fn query_swap_chain_support(&self, device: vk::VkPhysicalDevice) -> SwapChainSupportDetails {
        let mut details = SwapChainSupportDetails::default();
        unsafe {
            vk::vkGetPhysicalDeviceSurfaceCapabilitiesKHR(device, self.surface, &mut details.capabilities);
            let mut format_count:u32 = 0;
            vk::vkGetPhysicalDeviceSurfaceFormatsKHR(device, self.surface, &mut format_count, null_mut());
            if format_count > 0 {
                details.formats.resize(format_count as usize, vk::VkSurfaceFormatKHR::default());
                vk::vkGetPhysicalDeviceSurfaceFormatsKHR(device, self.surface, &mut format_count, details.formats.as_mut_ptr());
            }
            let mut present_mode_count:u32 = 0;
            vk::vkGetPhysicalDeviceSurfacePresentModesKHR(device, self.surface, &mut present_mode_count, null_mut());
            if present_mode_count > 0 {
                details.present_modes.resize(present_mode_count as usize, vk::VkPresentModeKHR::default());
                vk::vkGetPhysicalDeviceSurfacePresentModesKHR(device, self.surface, &mut present_mode_count, details.present_modes.as_mut_ptr());
            }
        }

        details
    }


    fn choose_swap_chain_format(&self, formats: &Vec<vk::VkSurfaceFormatKHR>) -> vk::VkSurfaceFormatKHR {
        for &format in formats {
            if format.format == vk::VK_FORMAT_B8G8R8A8_SRGB && format.colorSpace == vk::VK_COLOR_SPACE_SRGB_NONLINEAR_KHR {
                return format
            }
        }

        formats[0]
    }


    fn choose_swapchain_present_mode(&self, present_modes: &Vec<vk::VkPresentModeKHR>) -> vk::VkPresentModeKHR {
        for &present_mode in present_modes {
            if present_mode == vk::VK_PRESENT_MODE_MAILBOX_KHR {
                return present_mode;
            }
        }
        vk::VK_PRESENT_MODE_IMMEDIATE_KHR//VK_PRESENT_MODE_FIFO_KHR
    }


    fn choose_swap_extent(&self, capabilities: &vk::VkSurfaceCapabilitiesKHR) -> vk::VkExtent2D {
        if capabilities.currentExtent.width != u32::MAX {
            capabilities.currentExtent
        } else {
            let (mut width, mut height): (i32, i32) = (0, 0);
            unsafe {glfwGetFramebufferSize(self.window, &mut width, &mut height)}
            vk::VkExtent2D {
                width: (width as u32).clamp(capabilities.minImageExtent.width, capabilities.maxImageExtent.width),
                height: (height as u32).clamp(capabilities.minImageExtent.height, capabilities.maxImageExtent.height)
            }
        }
    }


    fn create_shader_module(&self, code: &Vec<u8>) -> vk::VkShaderModule {
        let mut create_info = vk::VkShaderModuleCreateInfo::default();
        create_info.sType = vk::VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
        create_info.codeSize = code.len();
        create_info.pCode = code.as_ptr() as _;
        let mut shader_module = vk::VkShaderModule::default();
        if unsafe{vk::vkCreateShaderModule(self.device, &create_info, null(), &mut shader_module)} != vk::VK_SUCCESS {
            panic!("failed to create shader module");
        }

        shader_module
    }
}


fn has_stencil_component(format: vk::VkFormat) -> bool {
    format == vk::VK_FORMAT_D32_SFLOAT_S8_UINT || format == vk::VK_FORMAT_D24_UNORM_S8_UINT
}


fn read_file(filename: &str) -> Vec<u8> {
    std::fs::read(filename).expect("failed to open file")
}


const fn get_api_version(major: u32, minor: u32) -> u32 {
    major << 22 | minor << 12
}


fn populate_debug_messenger_create_info(create_info: &mut vk::VkDebugUtilsMessengerCreateInfoEXT) {
    create_info.sType = vk::VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT;
    create_info.messageSeverity = (
        vk::VK_DEBUG_UTILS_MESSAGE_SEVERITY_VERBOSE_BIT_EXT | vk::VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT
        | vk::VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT) as _;
    create_info.messageType = (
        vk::VK_DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT | vk::VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT
        | vk::VK_DEBUG_UTILS_MESSAGE_TYPE_PERFORMANCE_BIT_EXT) as _;
    create_info.pfnUserCallback = Some(debugCallback);
    create_info.pUserData = null_mut() as _;
}


fn create_debug_utils_messenger(
    instance: vk::VkInstance, 
    p_create_info: *const vk::VkDebugUtilsMessengerCreateInfoEXT,
    p_allocator: *const vk::VkAllocationCallbacks,
    p_debug_messenger: *mut vk::VkDebugUtilsMessengerEXT
) -> vk::VkResult {
    let option_fn_ptr = unsafe{std::mem::transmute::<_, vk::PFN_vkCreateDebugUtilsMessengerEXT>(
        vk::vkGetInstanceProcAddr(instance, "vkCreateDebugUtilsMessengerEXT".as_ptr() as _))};
    if let Some(fn_ptr) = option_fn_ptr {
        unsafe{fn_ptr(instance, p_create_info, p_allocator, p_debug_messenger)}
    } else {
        vk::VK_ERROR_EXTENSION_NOT_PRESENT
    }
}


fn destroy_debug_utils_messenger(
    instance: vk::VkInstance, 
    p_debug_messenger: vk::VkDebugUtilsMessengerEXT,
    p_allocator: *const vk::VkAllocationCallbacks,
) {
    let option_fn_ptr = unsafe{std::mem::transmute::<_, vk::PFN_vkDestroyDebugUtilsMessengerEXT>(
        vk::vkGetInstanceProcAddr(instance, "vkDestroyDebugUtilsMessengerEXT".as_ptr() as _))};
    if let Some(fn_ptr) = option_fn_ptr {
        unsafe{fn_ptr(instance, p_debug_messenger, p_allocator)}
    } 
}

#[derive(Default)]
pub struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>,
    transfer_family: Option<u32>
}

impl QueueFamilyIndices {

    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some() && self.transfer_family.is_some()
    }
}


#[derive(Default)]
struct SwapChainSupportDetails {
    capabilities: vk::VkSurfaceCapabilitiesKHR,
    formats: Vec<vk::VkSurfaceFormatKHR>,
    present_modes: Vec<vk::VkPresentModeKHR>
}


#[allow(non_snake_case, unused_variables)]
#[unsafe(no_mangle)]
extern "C" fn debugCallback(
    messageSeverity: vk::VkDebugUtilsMessageSeverityFlagBitsEXT,
    messageType: vk::VkDebugUtilsMessageTypeFlagsEXT,
    pCallbackData: *const vk::VkDebugUtilsMessengerCallbackDataEXT,
    pUserData: *mut std::ffi::c_void
) -> vk::VkBool32 {
    eprintln!(
        "validation layer: {}", 
        unsafe{std::ffi::CStr::from_ptr((*pCallbackData).pMessage)}.to_str().unwrap()
    );

    return vk::VK_FALSE;
}

#[allow(non_snake_case, unused_variables)]
#[unsafe(no_mangle)]
extern "C" fn framebufferResizeCallback(window: *mut GLFWwindow, width: i32, height: i32) {
    unsafe {
        let app = glfwGetWindowUserPointer(window) as *mut HelloTriangleApplication;
        (*app).framebuffer_resized = true;
    }
}
