describe("Governance Test", () => {
  let committeeName: string
  let committeeId: string
  let customerId: string

  before(() => {
    const testEmail = `test-${Date.now()}@example.com`
    const testTelegramId = `user${Date.now()}`
    cy.createCustomer(testEmail, testTelegramId).then((id) => {
      customerId = id
      cy.log(`Created customer with ID: ${id}`)
    })
  })

  beforeEach(() => {
    cy.on("uncaught:exception", (err) => {
      if (err.message.includes("ResizeObserver loop")) {
        return false
      }
    })
  })

  it("should successfully create a Committees", () => {
    committeeName = `${Date.now()}`
    cy.visit("/committees")
    cy.wait(1000)
    cy.takeScreenshot("1_step-visit-committees")

    cy.get('[data-testid="global-create-button"]').click()
    cy.takeScreenshot("2_step-click-create-committee-button")

    cy.get('[data-testid="committee-create-name-input"]')
      .type(committeeName)
      .should("have.value", committeeName)
    cy.takeScreenshot("3_step-fill-committee-name")

    cy.get('[data-testid="committee-create-submit-button"]').click()
    cy.takeScreenshot("4_step-submit-committee-creation")

    cy.contains("Committee created successfully").should("be.visible")
    cy.takeScreenshot("5_step-committee-created-successfully")

    cy.getIdFromUrl("/committees/").then((id) => {
      committeeId = id
    })
  })

  it("should show newly added committee in the list", () => {
    cy.visit(`/committees`)
    cy.wait(1000)
    cy.contains(committeeName).should("be.visible")
    cy.takeScreenshot("6_step-view-committees-list")
  })

  it("should be able to add a new member to Committee", () => {
    cy.visit(`/committees/${committeeId}`)
    cy.wait(1000)
    cy.contains(committeeName).should("be.visible")
    cy.takeScreenshot("7_step-visit-committee-details")

    cy.get('[data-testid="committee-add-member-button"]').click()
    cy.takeScreenshot("8_step-click-add-member-button")

    cy.get('[data-testid="committee-add-user-select"]').should("be.visible").click()
    cy.get('[role="option"]')
      .contains("admin")
      .then((option) => {
        cy.wrap(option).click()
        cy.takeScreenshot("9_step-select-admin-role")
        cy.get('[data-testid="committee-add-user-submit-button"]').click()
        cy.takeScreenshot("10_step-submit-add-member")
        cy.contains("User added to committee successfully").should("be.visible")
        cy.takeScreenshot("11_step-verify-member-added")
        cy.contains(option.text().split(" ")[0]).should("be.visible")
      })
  })

  it("attach a committee to a policy", () => {
    cy.visit(`/policies`)
    cy.get('[data-testid="table-row-2"] > :nth-child(3) > a > .gap-2').should(
      "be.visible",
    )
    cy.takeScreenshot("12_step-visit-policies-page")

    cy.get('[data-testid="table-row-2"] > :nth-child(3) > a > .gap-2').click()
    cy.takeScreenshot("13_step-select-policy")

    cy.get('[data-testid="policy-assign-committee"]').click()
    cy.get('[data-testid="policy-select-committee-selector"]').click()
    cy.get('[role="option"]').contains(committeeName).click()
    cy.get("[data-testid=policy-assign-committee-threshold-input]").type("1")
    cy.takeScreenshot("14_step-assign-committee-to-policy")

    cy.get("[data-testid=policy-assign-committee-submit-button]").click()
    cy.contains("Committee assigned to policy successfully").should("be.visible")
    cy.takeScreenshot("15_step-verify-committee-assigned")
    cy.contains(committeeName).should("be.visible")
  })

  it("Pending actions should be visible in list", () => {
    const amount = 1000
    cy.createDeposit(amount, customerId).then(() => {
      cy.initiateWithdrawal(amount, customerId).then(() => {
        cy.visit(`/actions`)
        cy.get('[data-testid="table-row-0"] > :nth-child(4) > a > .gap-2').should(
          "be.visible",
        )
        cy.takeScreenshot("16_step-view-actions-page")
        cy.get('[data-testid="table-row-0"] > :nth-child(4) > a > .gap-2').click()

        cy.get("[data-testid=withdrawal-status-badge]")
          .should("be.visible")
          .should("have.text", "PENDING APPROVAL")
        cy.takeScreenshot("17_step-verify-pending-withdrawal")
      })
    })
  })

  it("Committee member should be able to approve a withdraw", () => {
    const amount = 1000
    cy.createDeposit(amount, customerId).then(() => {
      cy.initiateWithdrawal(amount, customerId).then((withdrawalId) => {
        cy.visit(`/withdrawals/${withdrawalId}`)
        cy.get("[data-testid=withdrawal-status-badge]").should("be.visible")
        cy.takeScreenshot("18_step-visit-withdrawal-details")

        cy.get("[data-testid=withdrawal-status-badge]").then((badge) => {
          if (badge.text() === "PENDING APPROVAL") {
            cy.get('[data-testid="approval-process-approve-button"]').click()
            cy.takeScreenshot("19_step-click-approve-button")

            cy.get('[data-testid="approval-process-dialog-approve-button"]').click()
            cy.contains("Approve Process").should("not.exist")
            cy.takeScreenshot("20_step-verify-approval-success")

            cy.get("[data-testid=withdrawal-status-badge]")
              .should("be.visible")
              .invoke("text")
              .should("eq", "PENDING CONFIRMATION")
          } else if (badge.text() === "PENDING CONFIRMATION") {
            throw new Error("State is Pending Confirmation")
          } else {
            throw new Error("Unexpected Withdraw State found")
          }
        })
      })
    })
  })

  it("Committee member should be able to deny a withdraw", () => {
    const amount = 1000
    cy.createDeposit(amount, customerId).then(() => {
      cy.initiateWithdrawal(amount, customerId).then((withdrawalId) => {
        cy.visit(`/withdrawals/${withdrawalId}`)
        cy.get("[data-testid=withdrawal-status-badge]").should("be.visible")
        cy.takeScreenshot("21_step-visit-withdrawal-for-denial")

        cy.get("[data-testid=withdrawal-status-badge]").then((badge) => {
          if (badge.text() === "PENDING APPROVAL") {
            cy.get('[data-testid="approval-process-deny-button"]').click()
            cy.takeScreenshot("22_step-click-deny-button")

            cy.get('[data-testid="approval-process-dialog-deny-reason"]').type("testing")
            cy.get('[data-testid="approval-process-dialog-deny-button"]').click()
            cy.contains("Deny Process").should("not.exist")
            cy.takeScreenshot("23_step-verify-denial-success")

            cy.get("[data-testid=withdrawal-status-badge]")
              .should("be.visible")
              .invoke("text")
              .should("eq", "DENIED")
          } else if (badge.text() === "PENDING CONFIRMATION") {
            throw new Error("State is Pending Confirmation")
          } else {
            throw new Error("Unexpected Withdraw State found")
          }
        })
      })
    })
  })
})