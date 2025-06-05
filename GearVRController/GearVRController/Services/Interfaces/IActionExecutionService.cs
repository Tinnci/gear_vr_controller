using GearVRController.Enums;
using System.Threading.Tasks;

namespace GearVRController.Services.Interfaces
{
    public interface IActionExecutionService
    {
        void ExecuteAction(GestureAction action);
    }
}