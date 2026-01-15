use glfw::ffi::*;
use vulkan::vulkan as vk;
use std::ptr::{null, null_mut};
use std::collections::{BTreeMap, HashSet};

const WIDTH:i32 = 800;
const HEIGHT:i32 = 600;

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

#[derive(Default)]
pub struct HelloTriangleApplication {
    window: *mut GLFWwindow,
    instance: vk::VkInstance,
    debug_messenger: vk::VkDebugUtilsMessengerEXT,
    physical_device: vk::VkPhysicalDevice,
    device: vk::VkDevice,
    graphics_queue: vk::VkQueue,
    surface: vk::VkSurfaceKHR,
    present_queue: vk::VkQueue,
    swap_chain: vk::VkSwapchainKHR,
    swap_chain_images: Vec<vk::VkImage>,
    swap_chain_image_format: vk::VkFormat,
    swap_chain_extent: vk::VkExtent2D,
    swap_chain_image_views: Vec<vk::VkImageView>,
    render_pass: vk::VkRenderPass,
    pipeline_layout: vk::VkPipelineLayout,
    pipeline: vk::VkPipeline,
    swap_chain_framebuffers: Vec<vk::VkFramebuffer>,
    command_pool: vk::VkCommandPool,
    command_buffer: vk::VkCommandBuffer,
    image_available_semaphore: vk::VkSemaphore,
    render_finished_semaphore: vk::VkSemaphore,
    in_flight: vk::VkFence
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
            glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);

            self.window = glfwCreateWindow(
                WIDTH, 
                HEIGHT, 
                "VK_app\0".as_ptr() as _, 
                std::ptr::null_mut(), 
                std::ptr::null_mut()
            );
        }
    }


    fn init_vulkan(&mut self) {
        self.create_instance();
        self.setup_debug_messanger();
        self.create_surface();
        self.pick_physical_device();
        self.create_logical_device();
        self.create_swapchain();
        self.create_image_views();
        self.create_render_pass();
        self.create_graphics_pipeline();
        self.create_framebuffers();
        self.create_command_pool();
        self.create_command_buffer();
        self.create_sync_objects();
    }

    

    fn  main_loop(&mut self) {
        unsafe {
            while glfwWindowShouldClose(self.window) == 0 {
                glfwPollEvents();
                self.draw_frame();
            }

            vk::vkDeviceWaitIdle(self.device);
        }
    }


    fn cleanup(&mut self) {
        unsafe {
            vk::vkDestroySemaphore(self.device, self.image_available_semaphore, null());
            vk::vkDestroySemaphore(self.device, self.render_finished_semaphore, null());
            vk::vkDestroyFence(self.device, self.in_flight, null());
            vk::vkDestroyCommandPool(self.device, self.command_pool, null());
            for frame_buffer in &self.swap_chain_framebuffers {
                vk::vkDestroyFramebuffer(self.device, *frame_buffer, null());
            }
            vk::vkDestroyPipeline(self.device, self.pipeline, null());
            vk::vkDestroyPipelineLayout(self.device, self.pipeline_layout, null());
            vk::vkDestroyRenderPass(self.device, self.render_pass, null());
            for image_view in &self.swap_chain_image_views {
                vk::vkDestroyImageView(self.device, *image_view, null());
            }
            vk::vkDestroySwapchainKHR(self.device, self.swap_chain, null());
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


    fn draw_frame(&mut self) {
        unsafe {
            vk::vkWaitForFences(self.device, 1, &self.in_flight, vk::VK_TRUE, u64::MAX);
            vk::vkResetFences(self.device, 1, &mut self.in_flight);

            let mut image_index: u32 = 0;
            vk::vkAcquireNextImageKHR(
                self.device,
                self.swap_chain, 
                u64::MAX, 
                self.image_available_semaphore, 
                null_mut(), 
                &mut image_index);
            vk::vkResetCommandBuffer(self.command_buffer, 0);
            self.record_command_buffer(self.command_buffer, image_index);

            let mut submit_info = vk::VkSubmitInfo::default();
            submit_info.sType = vk::VK_STRUCTURE_TYPE_SUBMIT_INFO;

            let wait_semaphores = [self.image_available_semaphore];
            let wait_stages = [vk::VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT];
            submit_info.waitSemaphoreCount = 1;
            submit_info.pWaitSemaphores = wait_semaphores.as_ptr();
            submit_info.pWaitDstStageMask = wait_stages.as_ptr() as _;
            submit_info.commandBufferCount = 1;
            submit_info.pCommandBuffers = &self.command_buffer;

            let signal_semaphores = [self.render_finished_semaphore];
            submit_info.signalSemaphoreCount = 1;
            submit_info.pSignalSemaphores = signal_semaphores.as_ptr();

            if vk::vkQueueSubmit(self.graphics_queue, 1, &submit_info, self.in_flight) != vk::VK_SUCCESS {
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

            vk::vkQueuePresentKHR(self.present_queue, &present_info);
        }
    }


    fn create_sync_objects(&mut self) {
        let mut semaphore_create_info = vk::VkSemaphoreCreateInfo::default();
        semaphore_create_info.sType = vk::VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
        let mut fence_create_info = vk::VkFenceCreateInfo::default();
        fence_create_info.sType = vk::VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
        fence_create_info.flags = vk::VK_FENCE_CREATE_SIGNALED_BIT as _;

        if unsafe {
            vk::vkCreateSemaphore(self.device, &semaphore_create_info, null(), &mut self.image_available_semaphore) |
            vk::vkCreateSemaphore(self.device, &semaphore_create_info, null(), &mut self.render_finished_semaphore) |
            vk::vkCreateFence(self.device, &fence_create_info, null(), &mut self.in_flight)
        } != vk::VK_SUCCESS {
            panic!("failed to create synchronization objects");
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
        let clear_color = vk::VkClearValue{color: vk::VkClearColorValue{float32: [0.0, 0.0, 0.0, 1.0]}};
        render_pass_info.clearValueCount = 1;
        render_pass_info.pClearValues = &clear_color;

        unsafe{
            vk::vkCmdBeginRenderPass(command_buffer, &render_pass_info, vk::VK_SUBPASS_CONTENTS_INLINE);
            vk::vkCmdBindPipeline(command_buffer, vk::VK_PIPELINE_BIND_POINT_GRAPHICS, self.pipeline);

            let mut viewport = vk::VkViewport::default();
            viewport.x = 0.0;
            viewport.y = 0.0;
            viewport.width = self.swap_chain_extent.width as _;
            viewport.height = self.swap_chain_extent.height as _;
            viewport.minDepth = 0.0;
            viewport.maxDepth = 0.0; 
            vk::vkCmdSetViewport(command_buffer, 0, 1, &viewport);

            let mut scissor = vk::VkRect2D::default();
            scissor.offset = vk::VkOffset2D{x:0, y:0};
            scissor.extent = self.swap_chain_extent;
            vk::vkCmdSetScissor(command_buffer, 0, 1, &scissor);
            vk::vkCmdDraw(command_buffer, 3, 1, 0, 0);
            vk::vkCmdEndRenderPass(command_buffer);

            if vk::vkEndCommandBuffer(command_buffer) != vk::VK_SUCCESS {
                panic!("failed to record command buffer");
            }
        }
    }


    fn create_command_buffer(&mut self) {
        let mut alloc_info = vk::VkCommandBufferAllocateInfo::default();
        alloc_info.sType = vk::VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
        alloc_info.commandPool = self.command_pool;
        alloc_info.level = vk::VK_COMMAND_BUFFER_LEVEL_PRIMARY;
        alloc_info.commandBufferCount = 1;

        if unsafe{vk::vkAllocateCommandBuffers(self.device, &alloc_info, &mut self.command_buffer)} != vk::VK_SUCCESS {
            panic!("failed to allocate command buffers");
        }
    }


    fn create_command_pool(&mut self) {
        let queue_families = self.get_queue_families(self.physical_device);

        let mut pool_info = vk::VkCommandPoolCreateInfo::default();
        pool_info.sType = vk::VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
        pool_info.flags = vk::VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT as _;
        pool_info.queueFamilyIndex = queue_families.graphics_family.unwrap();

        if unsafe{vk::vkCreateCommandPool(self.device, &pool_info, null(), &mut self.command_pool)} != vk::VK_SUCCESS {
            panic!("failed to create command pool");
        }
    }


    fn create_framebuffers(&mut self) {
        self.swap_chain_framebuffers.resize(self.swap_chain_image_views.len(), vk::VkFramebuffer::default());
        for i in 0..self.swap_chain_image_views.len() {
            let attachments = [self.swap_chain_image_views[i]];

            let mut framebuffer_create_info = vk::VkFramebufferCreateInfo::default();
            framebuffer_create_info.sType = vk::VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO;
            framebuffer_create_info.renderPass = self.render_pass;
            framebuffer_create_info.attachmentCount = 1;
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
        let mut color_attachment = vk::VkAttachmentDescription::default();
        color_attachment.format = self.swap_chain_image_format;
        color_attachment.samples = vk::VK_SAMPLE_COUNT_1_BIT;
        color_attachment.loadOp = vk::VK_ATTACHMENT_LOAD_OP_CLEAR;
        color_attachment.storeOp = vk::VK_ATTACHMENT_STORE_OP_STORE;
        color_attachment.stencilLoadOp = vk::VK_ATTACHMENT_LOAD_OP_DONT_CARE;
        color_attachment.stencilStoreOp = vk::VK_ATTACHMENT_STORE_OP_DONT_CARE;
        color_attachment.initialLayout = vk::VK_IMAGE_LAYOUT_UNDEFINED;
        color_attachment.finalLayout = vk::VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;

        let mut color_attachment_ref = vk::VkAttachmentReference::default();
        color_attachment_ref.attachment = 0;
        color_attachment_ref.layout = vk::VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

        let mut subpass = vk::VkSubpassDescription::default();
        subpass.pipelineBindPoint = vk::VK_PIPELINE_BIND_POINT_GRAPHICS;
        subpass.colorAttachmentCount = 1;
        subpass.pColorAttachments = &color_attachment_ref;

        let mut render_pass_info = vk::VkRenderPassCreateInfo::default();
        render_pass_info.sType = vk::VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO;
        render_pass_info.attachmentCount = 1;
        render_pass_info.pAttachments = &color_attachment;
        render_pass_info.subpassCount = 1;
        render_pass_info.pSubpasses = &subpass;

        let mut dependency = vk::VkSubpassDependency::default();
        dependency.srcSubpass = vk::VK_SUBPASS_EXTERNAL as _;
        dependency.dstSubpass = 0;
        dependency.srcStageMask = vk::VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT as _;
        dependency.srcAccessMask = 0;
        dependency.dstStageMask = vk::VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT as _;
        dependency.dstAccessMask = vk::VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT as _;

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
        vertex_input_info.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO;
        vertex_input_info.vertexBindingDescriptionCount = 0;
        vertex_input_info.pVertexBindingDescriptions = null();
        vertex_input_info.vertexAttributeDescriptionCount = 0;
        vertex_input_info.pVertexAttributeDescriptions = null();

        let mut input_assembly = vk::VkPipelineInputAssemblyStateCreateInfo::default();
        input_assembly.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO;
        input_assembly.topology = vk::VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST;
        input_assembly.primitiveRestartEnable = vk::VK_FALSE;


        let mut viewport = vk::VkViewport::default();
        viewport.x = 0.0;
        viewport.y = 0.0;
        viewport.width = self.swap_chain_extent.width as f32;
        viewport.height = self.swap_chain_extent.height as f32;
        viewport.minDepth = 0.0;
        viewport.maxDepth = 1.0;

        let mut scissor = vk::VkRect2D::default();
        scissor.offset = vk::VkOffset2D{x:0,y:0};
        scissor.extent = self.swap_chain_extent;

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
        rasterizer.frontFace = vk::VK_FRONT_FACE_CLOCKWISE;
        rasterizer.depthBiasEnable = vk::VK_FALSE;
        rasterizer.depthBiasConstantFactor = 0.0;
        rasterizer.depthBiasClamp = 0.0;
        rasterizer.depthBiasSlopeFactor = 0.0;

        let mut multisampling = vk::VkPipelineMultisampleStateCreateInfo::default();
        multisampling.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO;
        multisampling.sampleShadingEnable = vk::VK_FALSE;
        multisampling.rasterizationSamples = vk::VK_SAMPLE_COUNT_1_BIT;
        multisampling.minSampleShading = 1.0;
        multisampling.pSampleMask = null();
        multisampling.alphaToCoverageEnable = vk::VK_FALSE;
        multisampling.alphaToOneEnable = vk::VK_FALSE;

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
        color_blend.blendConstants[0] = 0.0;
        color_blend.blendConstants[1] = 0.0;
        color_blend.blendConstants[2] = 0.0;
        color_blend.blendConstants[3] = 0.0;

        let mut pipeline_layout_create_info = vk::VkPipelineLayoutCreateInfo::default();
        pipeline_layout_create_info.sType = vk::VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
        pipeline_layout_create_info.setLayoutCount = 0;
        pipeline_layout_create_info.pSetLayouts = null();
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

        let mut pipeline_info = vk::VkGraphicsPipelineCreateInfo::default();
        pipeline_info.sType = vk::VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
        pipeline_info.stageCount = 2;
        pipeline_info.pStages = shader_stages.as_ptr();
        pipeline_info.pVertexInputState = &vertex_input_info;
        pipeline_info.pInputAssemblyState = &input_assembly;
        pipeline_info.pViewportState = &viewport_state;
        pipeline_info.pRasterizationState = &rasterizer;
        pipeline_info.pMultisampleState = &multisampling;
        pipeline_info.pDepthStencilState = null();
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
        for (i, &image) in self.swap_chain_images.iter().enumerate() {
            let mut create_info = vk::VkImageViewCreateInfo::default();
            create_info.sType = vk::VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
            create_info.image = image;
            create_info.viewType = vk::VK_IMAGE_VIEW_TYPE_2D;
            create_info.format = self.swap_chain_image_format;
            create_info.components.r = vk::VK_COMPONENT_SWIZZLE_IDENTITY;
            create_info.components.g = vk::VK_COMPONENT_SWIZZLE_IDENTITY;
            create_info.components.b = vk::VK_COMPONENT_SWIZZLE_IDENTITY;
            create_info.components.a = vk::VK_COMPONENT_SWIZZLE_IDENTITY;
            create_info.subresourceRange.aspectMask = vk::VK_IMAGE_ASPECT_COLOR_BIT as _;
            create_info.subresourceRange.baseMipLevel = 0;
            create_info.subresourceRange.levelCount = 1;
            create_info.subresourceRange.baseArrayLayer = 0;
            create_info.subresourceRange.layerCount = 1;

            if unsafe{vk::vkCreateImageView(
                self.device, 
                &create_info, 
                null(), 
                &mut self.swap_chain_image_views[i]
            )} != vk::VK_SUCCESS {
                panic!("image view generation failed")
            }
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
        let queue_family_indices: [u32; 2] = [indices.graphics_family.unwrap(), indices.present_family.unwrap()];
        if indices.graphics_family != indices.present_family {
            create_info.imageSharingMode = vk::VK_SHARING_MODE_CONCURRENT;
            create_info.queueFamilyIndexCount = 2;
            create_info.pQueueFamilyIndices = queue_family_indices.as_ptr();
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
            [indices.graphics_family.unwrap(), indices.present_family.unwrap()]
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

        let physical_device_features = vk::VkPhysicalDeviceFeatures::default();

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

            if final_score == 0 {
                panic!("failed to find a suitable gpu");
            }
        }
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


            for (i, &family_properties) in queue_families.iter().enumerate() {
                if (family_properties.queueFlags & vk::VK_QUEUE_GRAPHICS_BIT as u32) > 0 {
                    indices.graphics_family = Some(i as u32);
                }

                let mut present_support: vk::VkBool32 = vk::VK_FALSE;
                vk::vkGetPhysicalDeviceSurfaceSupportKHR(
                    device, 
                    i as u32, 
                    self.surface, 
                    &mut present_support
                );

                if present_support == vk::VK_TRUE {
                    indices.present_family = Some(i as u32);
                }

                if indices.is_complete() {return indices}
            }
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
        vk::VK_PRESENT_MODE_FIFO_KHR
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


#[allow(non_snake_case)]
#[unsafe(no_mangle)]
extern "C" fn debugCallback(
    messageSeverity: vk::VkDebugUtilsMessageSeverityFlagBitsEXT,
    messageType: vk::VkDebugUtilsMessageTypeFlagsEXT,
    pCallbackData: *const vk::VkDebugUtilsMessengerCallbackDataEXT,
    pUserData: *mut std::ffi::c_void
) -> vk::VkBool32 {
    eprintln!("validation layer: {}", unsafe{std::ffi::CStr::from_ptr((*pCallbackData).pMessage)}.to_str().unwrap());

    return vk::VK_FALSE;
}


#[derive(Default)]
pub struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>
}

impl QueueFamilyIndices {

    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}


#[derive(Default)]
struct SwapChainSupportDetails {
    capabilities: vk::VkSurfaceCapabilitiesKHR,
    formats: Vec<vk::VkSurfaceFormatKHR>,
    present_modes: Vec<vk::VkPresentModeKHR>
}