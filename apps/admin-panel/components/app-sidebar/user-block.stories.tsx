import type { Meta, StoryObj } from "@storybook/react"
import { MockedProvider } from "@apollo/client/testing"

import { SidebarProvider } from "@lana/web/ui/sidebar"

import { UserBlock } from "./user-block"

import { Role, AvatarDocument } from "@/lib/graphql/generated"

const meta = {
  title: "Components/AppSidebar/UserBlock",
  component: UserBlock,
  parameters: {
    layout: "centered",
  },
  decorators: [
    (Story) => (
      <MockedProvider
        mocks={[
          {
            request: { query: AvatarDocument },
            result: {
              data: {
                me: {
                  user: {
                    userId: "usr_123",
                    email: "demo@example.com",
                    roles: [Role.Admin],
                  },
                },
              },
            },
          },
        ]}
      >
        <SidebarProvider>
          <div className="w-64 border rounded p-4">
            <Story />
          </div>
        </SidebarProvider>
      </MockedProvider>
    ),
  ],
} satisfies Meta<typeof UserBlock>

export default meta
type Story = StoryObj<typeof UserBlock>

export const Default: Story = {}
